//! Acyclic local loan identities and deferred values. A pending value is never
//! exposed to specifications or inhaled as a fact. Only a checked transfer can
//! construct the `ResolvedValue` used to publish it.

use prusti_rustc_interface::middle::mir;
use rustc_hash::FxHashMap;
use vir::CastType;

use super::loan_transfer::ResolvedValue;

#[derive(Default)]
pub(crate) struct LoanStates<'vir> {
    records: FxHashMap<mir::Location, LoanState<'vir>>,
    declarations: Vec<vir::Stmt<'vir>>,
    parents: FxHashMap<mir::Location, Vec<mir::Location>>,
}

/// The identity is the creation event, not the reference slot or target address.
#[derive(Clone, Copy)]
pub(crate) struct LoanState<'vir> {
    pub origin: mir::Location,
    tag: u128,
    pub target: vir::ExprRef<'vir>,
    entry: vir::ExprSnap<'vir>,
    deferred: vir::ExprSnap<'vir>,
    live: vir::ExprBool<'vir>,
    resolved: vir::ExprBool<'vir>,
    suspended: vir::ExprBool<'vir>,
    parent: vir::ExprInt<'vir>,
}

impl<'vir> LoanStates<'vir> {
    pub(crate) fn register(
        &mut self,
        vcx: &'vir vir::VirCtxt<'vir>,
        origin: mir::Location,
        snapshot: vir::TypeSnap<'vir>,
    ) {
        let name = format!(
            "_loan_at_{}_{}",
            origin.block.index(),
            origin.statement_index
        );
        macro_rules! local {
            ($role:literal, $ty:expr, $initial:expr) => {{
                let decl = vcx.mk_local_decl(vir::vir_format!(vcx, "{name}_{}", $role), $ty);
                self.declarations
                    .push(vcx.mk_local_decl_stmt(decl, $initial));
                vcx.mk_local_ex(decl)
            }};
        }
        let record = LoanState {
            origin,
            tag: self.records.len() as u128 + 1,
            target: local!("target", vir::TYPE_REF, None),
            entry: local!("entry", snapshot, None),
            deferred: local!("deferred", snapshot, None),
            live: local!("live", vir::TYPE_BOOL, Some(vcx.mk_bool::<false>())),
            resolved: local!("resolved", vir::TYPE_BOOL, Some(vcx.mk_bool::<false>())),
            suspended: local!("suspended", vir::TYPE_BOOL, Some(vcx.mk_bool::<false>())),
            parent: local!("parent", vir::TYPE_INT, Some(vcx.mk_uint::<0>())),
        };
        assert!(self.records.insert(origin, record).is_none());
    }

    pub(crate) fn get(&self, origin: mir::Location) -> Option<LoanState<'vir>> {
        self.records.get(&origin).copied()
    }

    pub(crate) fn declarations(&self) -> impl Iterator<Item = vir::Stmt<'vir>> + '_ {
        self.declarations.iter().copied()
    }

    pub(crate) fn set_parents(&mut self, origin: mir::Location, parents: Vec<mir::Location>) {
        assert!(self.parents.insert(origin, parents).is_none());
    }

    pub(crate) fn parents(&self, origin: mir::Location) -> Vec<LoanState<'vir>> {
        self.parents
            .get(&origin)
            .into_iter()
            .flatten()
            .map(|p| self.get(*p).expect("registered parent loan"))
            .collect()
    }
}

impl<'vir> LoanState<'vir> {
    pub(crate) fn begin(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        target: vir::ExprRef<'vir>,
        entry: vir::ExprSnap<'vir>,
        parents: &[(LoanState<'vir>, vir::ExprBool<'vir>)],
    ) -> Vec<vir::Stmt<'vir>> {
        let mut stmts = vec![
            vcx.mk_comment_stmt(vir::vir_format!(
                vcx,
                "[loan-state] begin {:?}",
                self.origin
            )),
            vcx.mk_assert_stmt(vcx.mk_eq_expr(self.live, vcx.mk_bool::<false>())),
            vcx.mk_assert_stmt(vcx.mk_eq_expr(self.resolved, vcx.mk_bool::<false>())),
            vcx.mk_pure_assign_stmt(self.target, target),
            vcx.mk_pure_assign_stmt(self.entry, entry),
            vcx.mk_pure_assign_stmt(self.live, vcx.mk_bool::<true>()),
        ];
        for (parent, guard) in parents {
            let link = vec![
                vcx.mk_assert_stmt(vcx.mk_eq_expr(self.parent, vcx.mk_uint::<0>())),
                vcx.mk_assert_stmt(parent.live),
                vcx.mk_assert_stmt(vcx.mk_eq_expr(parent.suspended, vcx.mk_bool::<false>())),
                vcx.mk_assert_stmt(vcx.mk_eq_expr(self.target, parent.target)),
                vcx.mk_pure_assign_stmt(parent.suspended, vcx.mk_bool::<true>()),
                vcx.mk_pure_assign_stmt(
                    self.parent,
                    vcx.mk_const_expr(vir::ConstData::Int(parent.tag))
                        .downcast_ty(),
                ),
            ];
            stmts.push(vcx.mk_if_stmt(*guard, vcx.alloc_slice(&link), &[]));
        }
        stmts
    }

    pub(crate) fn require_live(self, vcx: &'vir vir::VirCtxt<'vir>) -> Vec<vir::Stmt<'vir>> {
        vec![
            vcx.mk_assert_stmt(self.live),
            vcx.mk_assert_stmt(vcx.mk_eq_expr(self.suspended, vcx.mk_bool::<false>())),
            vcx.mk_assert_stmt(vcx.mk_eq_expr(self.resolved, vcx.mk_bool::<false>())),
        ]
    }

    pub(crate) fn finish(
        self,
        vcx: &'vir vir::VirCtxt<'vir>,
        resolved: ResolvedValue<'vir>,
        parents: &[LoanState<'vir>],
    ) -> Vec<vir::Stmt<'vir>> {
        let mut stmts = vec![
            vcx.mk_comment_stmt(vir::vir_format!(
                vcx,
                "[loan-state] resolve {:?}",
                self.origin
            )),
            vcx.mk_assert_stmt(vcx.mk_eq_expr(self.target, resolved.target())),
            vcx.mk_pure_assign_stmt(self.deferred, resolved.value()),
            vcx.mk_pure_assign_stmt(self.live, vcx.mk_bool::<false>()),
            vcx.mk_pure_assign_stmt(self.resolved, vcx.mk_bool::<true>()),
        ];
        for parent in parents {
            let guard = vcx.mk_eq_expr(
                self.parent,
                vcx.mk_const_expr(vir::ConstData::Int(parent.tag))
                    .downcast_ty::<vir::Int>(),
            );
            let resume = vec![
                vcx.mk_assert_stmt(parent.live),
                vcx.mk_assert_stmt(parent.suspended),
                vcx.mk_assert_stmt(vcx.mk_eq_expr(parent.target, self.target)),
                // This is the parent's current value at child expiry, not a
                // claim that later mutations leave it unchanged.
                vcx.mk_assert_stmt(
                    vcx.mk_eq_expr(resolved.current_at(parent.target), self.deferred),
                ),
                vcx.mk_pure_assign_stmt(parent.suspended, vcx.mk_bool::<false>()),
            ];
            stmts.push(vcx.mk_if_stmt(guard, vcx.alloc_slice(&resume), &[]));
        }
        stmts
    }
}
