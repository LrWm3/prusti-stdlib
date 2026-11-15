//! Checked one-element array transfers. No logical-index/address injectivity
//! is assumed. The existing split/join contracts remain trusted translation;
//! the matching frame predicate connects the two transitions.

use vir::CastType;

use super::loan_transfer::capture;
use crate::encoders::ty::{
    collection_segment::CollectionSegment, use_impure::TyUseImpure, use_pure::TyUsePure,
};

pub(crate) struct SliceReferentTransfer<'vir> {
    pub slots: SelectedElementTransfer<'vir>,
    pub referents: CollectionSegment<'vir>,
}

impl<'vir> SliceReferentTransfer<'vir> {
    pub fn split(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        counter: &mut usize,
        owner: vir::ExprRef<'vir>,
        index: vir::ExprInt<'vir>,
        length: vir::ExprInt<'vir>,
    ) -> Vec<vir::Stmt<'vir>> {
        let mut stmts =
            vec![vcx.mk_comment_stmt(
                "[referent-transfer] split actual referents and retain both regions",
            )];
        let owner = capture(vcx, counter, "referent_owner", owner, &mut stmts);
        let index = capture(vcx, counter, "referent_index", index, &mut stmts);
        let length = capture(vcx, counter, "referent_length", length, &mut stmts);
        let entry = self.slots.owner.ref_to_snap(owner).downcast_ty();
        let elements = capture(
            vcx,
            counter,
            "referent_elements",
            self.slots.model.expect_array().to_seq(entry, length),
            &mut stmts,
        );
        stmts.push(
            (self.referents.split)(
                elements,
                vcx.mk_int::<0>(),
                index,
                length,
                self.referents.params.ty_exprs(),
                self.referents.params.const_exprs(),
            )
            .alloc(),
        );
        stmts.extend(self.slots.split(vcx, counter, owner, index));
        stmts
    }

    pub fn join(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        counter: &mut usize,
        owner: vir::ExprRef<'vir>,
        index: vir::ExprInt<'vir>,
        length: vir::ExprInt<'vir>,
    ) -> Vec<vir::Stmt<'vir>> {
        let mut stmts =
            vec![vcx.mk_comment_stmt(
                "[referent-transfer] join current referent with preserved regions",
            )];
        let owner = capture(vcx, counter, "referent_owner", owner, &mut stmts);
        let index = capture(vcx, counter, "referent_index", index, &mut stmts);
        let length = capture(vcx, counter, "referent_length", length, &mut stmts);
        let array = self.slots.owner.expect_array();
        let model = self.slots.model.expect_array();
        let frame = capture(
            vcx,
            counter,
            "referent_frame",
            array.ref_to_index_frame_snap(owner, index),
            &mut stmts,
        );
        let elements = capture(
            vcx,
            counter,
            "referent_elements",
            model.to_seq(frame, length),
            &mut stmts,
        );
        // This rule restores scalar updates through an unchanged reference
        // slot. Reference replacement needs a separate old/new target rule.
        stmts.push(
            vcx.mk_assert_stmt(
                vcx.mk_eq_expr(
                    self.slots
                        .element
                        .ref_to_snap(array.ref_to_index_ref(owner, index)),
                    model.index(frame, index),
                ),
            ),
        );
        stmts.push(
            (self.referents.join)(
                elements,
                vcx.mk_int::<0>(),
                index,
                length,
                self.referents.params.ty_exprs(),
                self.referents.params.const_exprs(),
            )
            .alloc(),
        );
        let whole = self.slots.owner;
        stmts.extend(self.slots.join(vcx, counter, owner, index));
        stmts.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(
            model.to_seq(whole.ref_to_snap(owner).downcast_ty(), length),
            elements,
        )));
        stmts
    }
}

pub(crate) struct SelectedElementTransfer<'vir> {
    pub owner: TyUseImpure<'vir>,
    pub element: TyUseImpure<'vir>,
    pub model: TyUsePure<'vir>,
}

impl<'vir> SelectedElementTransfer<'vir> {
    pub(crate) fn split(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        counter: &mut usize,
        owner: vir::ExprRef<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> Vec<vir::Stmt<'vir>> {
        let mut stmts =
            vec![vcx.mk_comment_stmt("[element-transfer] split: freeze owner and index")];
        let owner = capture(vcx, counter, "array_owner", owner, &mut stmts);
        let index = capture(vcx, counter, "array_index", index, &mut stmts);
        let array = self.owner.expect_array();
        let target = capture(
            vcx,
            counter,
            "array_target",
            array.ref_to_index_ref(owner, index),
            &mut stmts,
        );
        stmts.push(vcx.mk_assert_stmt(self.owner.ref_to_pred(vcx, owner, None)));
        let entry = capture(
            vcx,
            counter,
            "array_entry",
            self.owner.ref_to_snap(owner),
            &mut stmts,
        );

        stmts.extend(self.owner.unfold(None, owner, Some(index), None, None));
        stmts.push(vcx.mk_assert_stmt(array.ref_to_index_frame_pred(owner, index)));
        stmts.push(vcx.mk_assert_stmt(self.element.ref_to_pred(vcx, target, None)));
        stmts.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(
            array.ref_to_index_frame_snap(owner, index),
            entry.downcast_ty(),
        )));
        stmts.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(
            self.element.ref_to_snap(target),
            self.model.expect_array().index(entry.downcast_ty(), index),
        )));
        stmts.push(vcx.mk_comment_stmt("[element-transfer] split: element and frame checked"));
        stmts
    }

    pub(crate) fn join(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        counter: &mut usize,
        owner: vir::ExprRef<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> Vec<vir::Stmt<'vir>> {
        let mut stmts =
            vec![vcx.mk_comment_stmt("[element-transfer] join: freeze current value and frame")];
        let owner = capture(vcx, counter, "array_owner", owner, &mut stmts);
        let index = capture(vcx, counter, "array_index", index, &mut stmts);
        let array = self.owner.expect_array();
        let target = capture(
            vcx,
            counter,
            "array_target",
            array.ref_to_index_ref(owner, index),
            &mut stmts,
        );
        stmts.push(vcx.mk_assert_stmt(array.ref_to_index_frame_pred(owner, index)));
        stmts.push(vcx.mk_assert_stmt(self.element.ref_to_pred(vcx, target, None)));
        let value = capture(
            vcx,
            counter,
            "array_current",
            self.element.ref_to_snap(target),
            &mut stmts,
        );
        let frame = capture(
            vcx,
            counter,
            "array_frame",
            array.ref_to_index_frame_snap(owner, index),
            &mut stmts,
        );

        stmts.extend(self.owner.fold(None, owner, Some(index), None, None));
        stmts.push(vcx.mk_assert_stmt(self.owner.ref_to_pred(vcx, owner, None)));
        let restored = self.owner.ref_to_snap(owner).downcast_ty();
        let model = self.model.expect_array();
        stmts.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(model.index(restored, index), value)));

        // The remainder is keyed by its logical owner/index, not by an assumed
        // inequality of physical element addresses (which need not hold for ZSTs).
        let other_decl = vcx.mk_local_decl("other_index", vir::TYPE_INT);
        let other = vcx.mk_local_ex(other_decl);
        let restored_element = model.index_generic(restored, other);
        let preserved = vcx.mk_eq_expr(restored_element, model.index_generic(frame, other));
        let different = vcx
            .mk_bin_op_expr(vir::BinOpKind::CmpNe, other, index)
            .downcast_ty::<vir::Bool>();
        let frame_relation = vcx
            .mk_bin_op_expr(vir::BinOpKind::Implies, different, preserved)
            .downcast_ty();
        stmts.push(vcx.mk_assert_stmt(vcx.mk_forall_expr(
            vcx.alloc_slice(&[other_decl]),
            vcx.alloc_slice(&[vcx.mk_trigger(&[restored_element])]),
            frame_relation,
        )));
        stmts.push(
            vcx.mk_comment_stmt("[element-transfer] join: selected value and remainder checked"),
        );
        stmts
    }
}
