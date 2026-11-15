//! Checked one-element array transfers. No logical-index/address injectivity
//! is assumed. The existing split/join contracts remain trusted translation;
//! the matching frame predicate connects the two transitions.

use vir::CastType;

use super::loan_transfer::capture;
use crate::encoders::ty::{use_impure::TyUseImpure, use_pure::TyUsePure};

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
