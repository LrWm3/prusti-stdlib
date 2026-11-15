//! Local, checked representation transfers. See STDLIB_OWNERSHIP_CORE.md.
//!
//! A captured value is an observation, not a permission or an unresolved
//! prophecy. No facts are inhaled here. PCG still schedules the transfer and
//! Viper checks its resource and value obligations at the native Rust use site.

use prusti_rustc_interface::middle::ty::{Ty, TyKind};
use vir::{CastType, CompType};

use super::loan_state::LoanState;
use crate::encoders::ty::{
    use_impure::{TyUseImpure, TyUseImpureMutRef},
    use_pure::TyUsePureMutRef,
};

pub(crate) fn enabled_for(ty: Ty<'_>) -> bool {
    prusti_utils::config::checked_scalar_loans()
        && matches!(
            ty.kind(),
            TyKind::Bool | TyKind::Char | TyKind::Int(_) | TyKind::Uint(_) | TyKind::Ref(..)
        )
}

/// The resource correspondence for one local loan. The slot and target have
/// distinct roles; no assumption of address injectivity is needed.
pub(crate) struct ScalarLoanTransfer<'vir> {
    pub concrete: TyUseImpure<'vir>,
    pub generic: TyUseImpure<'vir>,
    pub reference: TyUseImpureMutRef<'vir>,
    pub value_cast: TyUsePureMutRef<'vir>,
}

/// A witness is assigned once during the dynamic transfer. It is not persisted
/// across a call, loop backedge, branch join or loan lifetime.
struct ValueVersion<'vir>(vir::ExprSnap<'vir>);

/// Only produced after lowering the checked restoration. Pending loans cannot
/// construct this or expose their deferred value through this interface.
pub(crate) struct ResolvedValue<'vir> {
    target: vir::ExprRef<'vir>,
    value: vir::ExprSnap<'vir>,
    concrete: TyUseImpure<'vir>,
}

impl<'vir> ResolvedValue<'vir> {
    pub(crate) fn target(&self) -> vir::ExprRef<'vir> {
        self.target
    }
    pub(crate) fn value(&self) -> vir::ExprSnap<'vir> {
        self.value
    }
    pub(crate) fn current_at(&self, target: vir::ExprRef<'vir>) -> vir::ExprSnap<'vir> {
        self.concrete.ref_to_snap(target)
    }
}

/// Consumed by `acquire` or `resolve`; callers cannot silently reuse this
/// transfer's witness for another ownership transition.
pub(crate) struct CapturedTransfer<'vir> {
    rule: ScalarLoanTransfer<'vir>,
    target: vir::ExprRef<'vir>,
    value: ValueVersion<'vir>,
}

impl<'vir> ScalarLoanTransfer<'vir> {
    fn capture(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        counter: &mut usize,
        target: vir::ExprRef<'vir>,
        resolving: bool,
    ) -> (CapturedTransfer<'vir>, Vec<vir::Stmt<'vir>>) {
        let mut stmts = vec![vcx.mk_comment_stmt(if resolving {
            "[loan-transfer] resolve: capture current target and value"
        } else {
            "[loan-transfer] acquire: capture entry target and value"
        })];
        let target = capture(vcx, counter, "target", target, &mut stmts);
        let source = if resolving {
            self.generic
        } else {
            self.concrete
        };
        stmts.push(vcx.mk_assert_stmt(source.ref_to_pred(vcx, target, None)));
        let value = capture(
            vcx,
            counter,
            "value",
            source.ref_to_snap(target),
            &mut stmts,
        );
        (
            CapturedTransfer {
                rule: self,
                target,
                value: ValueVersion(value),
            },
            stmts,
        )
    }

    pub(crate) fn capture_entry(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        counter: &mut usize,
        target: vir::ExprRef<'vir>,
    ) -> (CapturedTransfer<'vir>, Vec<vir::Stmt<'vir>>) {
        self.capture(vcx, counter, target, false)
    }

    pub(crate) fn capture_current(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        counter: &mut usize,
        target: vir::ExprRef<'vir>,
    ) -> (CapturedTransfer<'vir>, Vec<vir::Stmt<'vir>>) {
        self.capture(vcx, counter, target, true)
    }
}

impl<'vir> CapturedTransfer<'vir> {
    pub(crate) fn remember(
        &self,
        vcx: &'vir vir::VirCtxt<'vir>,
        state: LoanState<'vir>,
        parents: &[(LoanState<'vir>, vir::ExprBool<'vir>)],
    ) -> Vec<vir::Stmt<'vir>> {
        state.begin(vcx, self.target, self.value.0, parents)
    }
    pub(crate) fn target(&self) -> vir::ExprRef<'vir> {
        self.target
    }

    pub(crate) fn acquire(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        slot: vir::ExprRef<'vir>,
    ) -> Vec<vir::Stmt<'vir>> {
        let mut stmts =
            vec![vcx.mk_assert_stmt(
                vcx.mk_eq_expr(self.rule.reference.deref(slot, None), self.target),
            )];
        stmts.extend(self.rule.reference.acquire_at(self.target));
        stmts.push(vcx.mk_assert_stmt(self.rule.generic.ref_to_pred(vcx, self.target, None)));
        stmts.push(
            vcx.mk_assert_stmt(
                vcx.mk_eq_expr(
                    self.rule.generic.ref_to_snap(self.target),
                    self.rule
                        .value_cast
                        .cast_to_callee_ctx(self.value.0)
                        .upcast_ty(),
                ),
            ),
        );
        stmts.push(vcx.mk_comment_stmt("[loan-transfer] acquire: correspondence checked"));
        stmts
    }

    pub(crate) fn resolve(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        owner_target: vir::ExprRef<'vir>,
    ) -> Vec<vir::Stmt<'vir>> {
        self.resolve_with_value(vcx, owner_target).0
    }

    pub(crate) fn resolve_with_value(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        owner_target: vir::ExprRef<'vir>,
    ) -> (Vec<vir::Stmt<'vir>>, ResolvedValue<'vir>) {
        let value = self
            .rule
            .value_cast
            .cast_to_caller_ctx(self.value.0.downcast_ty());
        let mut stmts = vec![vcx.mk_assert_stmt(vcx.mk_eq_expr(self.target, owner_target))];
        stmts.extend(self.rule.reference.resolve_at(self.target));
        stmts.push(vcx.mk_assert_stmt(self.rule.concrete.ref_to_pred(vcx, self.target, None)));
        stmts.push(
            vcx.mk_assert_stmt(
                vcx.mk_eq_expr(
                    self.rule.concrete.ref_to_snap(self.target),
                    self.rule
                        .value_cast
                        .cast_to_caller_ctx(self.value.0.downcast_ty()),
                ),
            ),
        );
        stmts.push(vcx.mk_comment_stmt("[loan-transfer] resolve: correspondence checked"));
        (
            stmts,
            ResolvedValue {
                target: self.target,
                value,
                concrete: self.rule.concrete,
            },
        )
    }
}

fn capture<'vir, T: CompType>(
    vcx: &'vir vir::VirCtxt<'vir>,
    counter: &mut usize,
    role: &str,
    value: vir::Expr<'vir, T>,
    stmts: &mut Vec<vir::Stmt<'vir>>,
) -> vir::Expr<'vir, T> {
    let name = vir::vir_format!(vcx, "_loan_{role}_{}", *counter);
    *counter += 1;
    let local = vcx.mk_local_decl(name, value.ty());
    stmts.push(vcx.mk_local_decl_stmt(local, Some(value)));
    vcx.mk_local_ex(local)
}
