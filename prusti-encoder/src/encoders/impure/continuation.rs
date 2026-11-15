//! Checked expiry observations for an existing call continuation.
//!
//! The original wand remains the authority for both resources and guarantees.
//! Its bound `old[lhs]` observations become available outside the wand only
//! after application, checked against the saved application heap.

use vir::{Foldable, Folder};

pub(crate) struct ContinuationSummary<'vir> {
    pub wand: vir::Wand<'vir>,
    pub entry: &'vir str,
    pub post_call: &'vir str,
}

impl<'vir> ContinuationSummary<'vir> {
    pub(crate) fn apply(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        expiry: &'vir str,
    ) -> Vec<vir::Stmt<'vir>> {
        let restored = self
            .wand
            .rhs
            .fold_with(&mut ExpiryObservation { vcx, expiry })
            .unwrap_or(self.wand.rhs);
        vec![
            vcx.mk_comment_stmt(vir::vir_format!(
                vcx,
                "[continuation] apply: entry={}, post={}, expiry={}",
                self.entry,
                self.post_call,
                expiry
            )),
            vcx.mk_apply_stmt(self.wand),
            vcx.mk_assert_stmt(restored),
            vcx.mk_comment_stmt(
                "[continuation] restored resources and expiry observations checked",
            ),
        ]
    }
}

struct ExpiryObservation<'vir> {
    vcx: &'vir vir::VirCtxt<'vir>,
    expiry: &'vir str,
}

impl<'vir> Folder<'vir, (), !> for ExpiryObservation<'vir> {
    fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
        if let vir::ExprKindData::Old(old) = expr.kind
            && old.label == vir::OldLabel::Lhs
        {
            return Some(self.vcx.mk_local_labelled_old_expr(old.expr, self.expiry));
        }
        expr.super_fold_with(self)
    }

    // A nested wand binds its own `lhs`; never capture that observation.
    fn fold_wand(&mut self, _: vir::Wand<'vir>) -> Option<vir::Wand<'vir>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vir::CastType;

    #[test]
    fn expiry_substitution_preserves_other_histories_and_nested_binders() {
        vir::init_vcx(vir::VirCtxt::new_without_tcx());
        vir::with_vcx(|vcx| {
            let value = vcx.mk_local_ex::<(), !, _>(vcx.mk_local_decl("value", vir::TYPE_INT));
            let entry = vcx.mk_old_expr(value);
            let post = vcx.mk_local_labelled_old_expr(value, "post");
            let future = vcx.mk_old_lhs_expr(value);
            let nested =
                vcx.mk_wand_expr(vcx.mk_wand(vcx.mk_bool::<true>(), vcx.mk_eq_expr(future, post)));
            let mut folder = ExpiryObservation {
                vcx,
                expiry: "expiry",
            };
            assert!(entry.fold_with(&mut folder).is_none());
            assert!(post.fold_with(&mut folder).is_none());
            assert!(nested.fold_with(&mut folder).is_none());
            let rewritten = future.fold_with(&mut folder).unwrap();
            let vir::ExprKindData::Old(old) = rewritten.kind else {
                panic!("expected old")
            };
            assert!(old.label == vir::OldLabel::Label("expiry"));
            assert!(std::ptr::eq(old.expr, value.as_dyn()));
        });
    }
}
