//! Resolve current resource observations through an explicitly matching owner.
//! Every branch reads an existing predicate; this pass grants no permissions.

use vir::{CastType, Foldable, Folder};

pub(crate) fn has_current_observation(expr: vir::ExprDyn<'_>) -> bool {
    struct Find(bool);
    impl<'vir> Folder<'vir, (), !> for Find {
        fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
            match expr.kind {
                vir::ExprKindData::Old(_) => None,
                vir::ExprKindData::FuncApp(app) if app.target == "ResourceReference" => {
                    self.0 = true;
                    None
                }
                _ => expr.super_fold_with(self),
            }
        }
        fn fold_wand(&mut self, _: vir::Wand<'vir>) -> Option<vir::Wand<'vir>> {
            None
        }
    }
    let mut find = Find(false);
    expr.fold_with(&mut find);
    find.0
}

pub(crate) struct CurrentResourceOwner<'vir> {
    pub address: vir::ExprRef<'vir>,
    pub ty: vir::ExprTyVal<'vir>,
    pub value: vir::ExprPSnap<'vir>,
}

pub(crate) struct CurrentResourceObservations<'a, 'vir> {
    pub vcx: &'vir vir::VirCtxt<'vir>,
    pub owners: &'a [CurrentResourceOwner<'vir>],
}

impl<'vir> Folder<'vir, (), !> for CurrentResourceObservations<'_, 'vir> {
    fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
        // Current ownership cannot justify a read in a historical heap.
        if matches!(expr.kind, vir::ExprKindData::Old(_)) {
            return None;
        }
        let folded = expr.super_fold_with(self);
        let current = folded.unwrap_or(expr);
        let vir::ExprKindData::FuncApp(state) = current.kind else {
            return folded;
        };
        if state.target != "ResourceReference" || self.owners.is_empty() {
            return folded;
        }
        let vir::ExprKindData::FuncApp(read) = state.args[0].kind else {
            return folded;
        };
        if read.target != "p_Param_snap" || read.args.len() != 2 {
            return folded;
        }
        let mut value = state.args[0].downcast_ty::<vir::PSnap>();
        for owner in self.owners.iter().rev() {
            let same = self.vcx.mk_conj(&[
                self.vcx.mk_eq_expr(read.args[0], owner.address.as_dyn()),
                self.vcx.mk_eq_expr(read.args[1], owner.ty.as_dyn()),
            ]);
            value = self.vcx.mk_ternary_expr(same, owner.value, value);
        }
        let state_ty = super::super::ty::resource_observation::state_type();
        Some(
            vir::FunctionIdn::<(vir::PSnap, vir::CSnap), vir::CSnap>::new(
                vir::ViperIdent::new("ResourceReference"),
                (vir::TYPE_PSNAP, state_ty),
                state_ty,
            )
            .call()(value, state.args[1].downcast_ty())
            .as_dyn(),
        )
    }

    fn fold_wand(&mut self, _: vir::Wand<'vir>) -> Option<vir::Wand<'vir>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vir::Foldable;

    #[test]
    fn owner_resolution_preserves_histories_wands_and_unmarked_reads() {
        vir::init_vcx(vir::VirCtxt::new_without_tcx());
        vir::with_vcx(|vcx| {
            let address = vcx.mk_local_ex(vcx.mk_local_decl("owner", vir::TYPE_REF));
            let ty = vcx.mk_local_ex(vcx.mk_local_decl("owner_type", vir::TYPE_TYVAL));
            let value = vcx.mk_local_ex(vcx.mk_local_decl("current_value", vir::TYPE_PSNAP));
            let state_ty = super::super::super::ty::resource_observation::state_type();
            let nested = vcx.mk_local_ex(vcx.mk_local_decl("nested", state_ty));
            let read = vir::FunctionIdn::<(vir::Ref, vir::TyVal), vir::PSnap>::new(
                vir::ViperIdent::new("p_Param_snap"),
                (vir::TYPE_REF, vir::TYPE_TYVAL),
                vir::TYPE_PSNAP,
            )
            .call()(address, ty);
            let state = vir::FunctionIdn::<(vir::PSnap, vir::CSnap), vir::CSnap>::new(
                vir::ViperIdent::new("ResourceReference"),
                (vir::TYPE_PSNAP, state_ty),
                state_ty,
            )
            .call()(read, nested);
            let owners = [CurrentResourceOwner { address, ty, value }];
            let mut folder = CurrentResourceObservations {
                vcx,
                owners: &owners,
            };
            assert!(has_current_observation(state.as_dyn()));
            assert!(!has_current_observation(vcx.mk_old_expr(state).as_dyn()));
            assert!(state.fold_with(&mut folder).is_some());
            assert!(read.fold_with(&mut folder).is_none());
            assert!(vcx.mk_old_expr(state).fold_with(&mut folder).is_none());
            assert!(
                vcx.mk_local_labelled_old_expr(state, "entry")
                    .fold_with(&mut folder)
                    .is_none()
            );
            assert!(vcx.mk_old_lhs_expr(state).fold_with(&mut folder).is_none());
            let wand =
                vcx.mk_wand_expr(vcx.mk_wand(vcx.mk_bool::<true>(), vcx.mk_eq_expr(state, nested)));
            assert!(wand.fold_with(&mut folder).is_none());
        });
    }
}
