use crate::encoders::{
    EncodeResult, ImpureEncVisitor, MirLocalDefEncOutput, MirSpecEnc, Pure,
    mir_fn::RustSignature,
    mir_pure::PostCapture,
    mir_shared::PureRvalueEnc,
    pure::spec::{EncodedPledge, MirSpecEncMode, PledgeArgs, PledgeExpr},
    ty::{
        RustTyDecomposition,
        generics::{GArgs, GArgsCastEnc, GArgsTyEnc, GParams, GenericParamsEnc},
        indirect::{
            ExprInput, IndirectPredicatesEnc, SnapshotBindings, projection_for_generalized_idx,
        },
        resource_boundary::{ResourceBoundaryEnc, ResourceBoundaryTask},
        resource_conversion::ResourceConversionEnc,
    },
};
use pcg::{
    borrow_pcg::{
        FunctionData, FunctionShape, FunctionShapeInput, FunctionShapeNode, FunctionShapeOutput,
        GeneralizedLifetime, MakeFunctionShapeError,
        borrow_pcg_edge::{BlockedNode, BorrowPcgEdge},
        edge::kind::BorrowPcgEdgeKind,
        region_projection::{ExtractRegionsCtxt, Generalized, LifetimeProjection, PcgRegion},
        state::BorrowsState,
        unblock_graph::UnblockGraph,
    },
    utils::place::remote::RemotePlace,
};
use prusti_interface::PrustiError;
use prusti_rustc_interface::{
    data_structures::fx::{FxHashMap, FxHashSet},
    index::{Idx, IndexVec},
    middle::{mir, ty},
    span::def_id::DefId,
};
use task_encoder::{EncodeFullError, EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::{CastType, Foldable, Folder, HasType};

pub(crate) type CallWand<'vir> = (
    usize,
    vir::Wand<'vir>,
    Vec<vir::Stmt<'vir>>,
    Vec<vir::Stmt<'vir>>,
    Vec<OwnerHandoff<'vir>>,
    Vec<crate::encoders::ty::resource_conversion::ScalarResourceView<'vir>>,
);

/// Observations made together at one call share their captured value. In
/// particular, an ADT projection of a captured snapshot must remain a
/// projection of that value when the loop forgets unrelated ghost locals.
#[derive(Default)]
struct CallSnapshots<'vir> {
    slot: usize,
    values: Vec<(vir::ExprDyn<'vir>, vir::ExprDyn<'vir>)>,
}

struct SnapshotSubstitute<'vir>(FxHashMap<&'vir str, vir::ExprDyn<'vir>>);

impl<'vir> Folder<'vir, (), !> for SnapshotSubstitute<'vir> {
    fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
        if let vir::ExprKindData::Local(local) = e.kind {
            self.0.get(local.name).copied()
        } else {
            e.super_fold_with(self)
        }
    }
}

impl<'vir> CallSnapshots<'vir> {
    /// A consequence of a functional postcondition that refers only to
    /// captured values. Dropping an unstable conjunct or weakening either
    /// branch is sound; antecedents and negated expressions stay intact.
    fn stable_consequence(
        vcx: &'vir vir::VirCtxt<'vir>,
        fact: vir::ExprBool<'vir>,
    ) -> vir::ExprBool<'vir> {
        use vir::{BinOpKind as B, ExprKindData as K};
        struct Stable(bool);
        impl<'vir> Folder<'vir, (), !> for Stable {
            fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                match e.kind {
                    K::Old(_)
                    | K::Field(..)
                    | K::PredicateApp(_)
                    | K::PredicatePerm(_)
                    | K::Unfolding(_)
                    | K::AccField(_)
                    | K::Wand(_)
                    | K::InhaleExhale(_) => {
                        self.0 = false;
                    }
                    K::FuncApp(app)
                        if app.target.starts_with("p_")
                            || app.target.starts_with("indexed_")
                            || app.target.starts_with("indirect_") =>
                    {
                        self.0 = false
                    }
                    _ => {}
                }
                e.super_fold_with(self)
            }
        }
        let stable = |expr: vir::ExprDyn<'vir>| {
            let mut check = Stable(true);
            let _ = expr.fold_with(&mut check);
            check.0
        };
        match fact.kind {
            K::BinOp(op) if matches!(op.kind, B::And | B::Or) => vcx
                .mk_bin_op_expr(
                    op.kind,
                    Self::stable_consequence(vcx, op.lhs.downcast_ty()),
                    Self::stable_consequence(vcx, op.rhs.downcast_ty()),
                )
                .downcast_ty(),
            K::BinOp(op) if op.kind == B::Implies && stable(op.lhs) => vcx
                .mk_bin_op_expr(
                    B::Implies,
                    op.lhs.downcast_ty::<vir::Bool>(),
                    Self::stable_consequence(vcx, op.rhs.downcast_ty()),
                )
                .downcast_ty(),
            K::Ternary(branch) if stable(branch.cond.as_dyn()) => vcx.mk_ternary_expr(
                branch.cond,
                Self::stable_consequence(vcx, branch.then.downcast_ty()),
                Self::stable_consequence(vcx, branch.else_.downcast_ty()),
            ),
            K::Let(binding) if stable(binding.val) => vcx.mk_let_expr(
                vcx.mk_local_decl(binding.name, binding.val.ty()),
                binding.val,
                Self::stable_consequence(vcx, binding.expr.downcast_ty()),
            ),
            _ if stable(fact.as_dyn()) => fact,
            _ => vcx.mk_bool::<true>(),
        }
    }

    fn bind<E: TaskEncoder>(
        &mut self,
        bindings: &SnapshotBindings<'vir>,
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
    ) -> SnapshotSubstitute<'vir> {
        let mut substitute = SnapshotSubstitute(FxHashMap::default());
        for &(decl, value) in bindings.borrow().iter() {
            let value = value.fold_with(&mut substitute).unwrap_or(value);
            let captured = self.capture(value.as_dyn(), visitor);
            substitute.0.insert(decl.name, captured);
        }
        substitute
    }

    fn bind_used<E: TaskEncoder>(
        &mut self,
        bindings: &SnapshotBindings<'vir>,
        facts: &[vir::ExprBool<'vir>],
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
    ) -> SnapshotSubstitute<'vir> {
        struct Uses<'vir>(FxHashSet<&'vir str>);
        impl<'vir> Folder<'vir, (), !> for Uses<'vir> {
            fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                if let vir::ExprKindData::Local(local) = expr.kind {
                    self.0.insert(local.name);
                }
                expr.super_fold_with(self)
            }
        }
        let mut used = Uses(FxHashSet::default());
        for fact in facts {
            let _ = fact.fold_with(&mut used);
        }
        // Bindings are dependency ordered. Walk backwards to include the
        // observations used by another retained observation, then emit forwards.
        for (decl, value) in bindings.borrow().iter().rev() {
            if used.0.contains(decl.name) {
                let _ = value.fold_with(&mut used);
            }
        }
        let mut substitute = SnapshotSubstitute(FxHashMap::default());
        for &(decl, value) in bindings.borrow().iter() {
            if used.0.contains(decl.name) {
                let value = value.fold_with(&mut substitute).unwrap_or(value);
                let captured = self.capture(value.as_dyn(), visitor);
                substitute.0.insert(decl.name, captured);
            }
        }
        substitute
    }

    fn same(a: vir::ExprDyn<'vir>, b: vir::ExprDyn<'vir>) -> bool {
        use vir::ExprKindData as K;
        if a.ty() != b.ty() {
            return false;
        }
        if std::ptr::eq(a.kind, b.kind) {
            return true;
        }
        match (a.kind, b.kind) {
            (K::Local(a), K::Local(b)) => a.name == b.name && a.ty == b.ty,
            (K::Const(a), K::Const(b)) => a == b,
            (K::Old(a), K::Old(b)) => a.label == b.label && Self::same(a.expr, b.expr),
            (K::FuncApp(a), K::FuncApp(b)) => {
                a.target == b.target
                    && a.reveal == b.reveal
                    && a.result_ty == b.result_ty
                    && a.typ_var_map == b.typ_var_map
                    && a.args.len() == b.args.len()
                    && a.args.iter().zip(b.args).all(|(a, b)| Self::same(a, b))
            }
            (K::AdtDestructor(a, ad), K::AdtDestructor(b, bd)) => ad == bd && Self::same(a, b),
            _ => false,
        }
    }

    fn capture<E: TaskEncoder>(
        &mut self,
        value: vir::ExprDyn<'vir>,
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
    ) -> vir::ExprDyn<'vir> {
        if let Some((_, saved)) = self
            .values
            .iter()
            .find(|(read, _)| Self::same(*read, value))
        {
            return saved;
        }
        let saved = if let vir::ExprKindData::Old(old) = value.kind
            && let vir::ExprKindData::AdtDestructor(base, field) = old.expr.kind
        {
            // ADT destructors are heap independent. Capture their base on
            // the same old heap, then keep the projection explicit.
            let read = visitor.vcx.maybe_apply_label(*base, Some(old.label));
            field.call()(self.capture(read, visitor))
        } else {
            let local = visitor.call_snapshot_local(&mut self.slot, value.ty());
            visitor.stmt(visitor.vcx.mk_pure_assign_stmt(local, value));
            local
        };
        self.values.push((value, saved));
        saved
    }
}

/// A restored mutable owner exposes its current nested resources while retaining
/// the checked obligation to restore the entry resources at the owner's expiry.
#[derive(Clone, Copy)]
struct OwnerIdentity<'vir> {
    address: vir::ExprRef<'vir>,
    ty: vir::ExprTyVal<'vir>,
}

#[derive(Clone)]
pub(crate) struct OwnerHandoff<'vir> {
    argument: mir::Local,
    owner: OwnerIdentity<'vir>,
    group: usize,
    current: vir::ExprBool<'vir>,
    entry: vir::ExprBool<'vir>,
    bindings: Vec<(vir::LocalDeclSnap<'vir>, vir::ExprSnap<'vir>)>,
}

#[derive(Clone)]
pub(crate) struct RestoredOwnerHandoff<'vir> {
    origin: Option<mir::Local>,
    selection: Option<vir::ExprInt<'vir>>,
    owner: OwnerIdentity<'vir>,
    group: usize,
    continuation: vir::Wand<'vir>,
    projection_path: Option<&'vir [&'vir str]>,
    guards: Vec<vir::ExprBool<'vir>>,
}

impl<'vir> RestoredOwnerHandoff<'vir> {
    /// A field's opaque group is restored as part of the enclosing input group.
    /// Keep the actual continuation unchanged; only its selection identity is
    /// the original whole owner, captured before the call. Viper still checks
    /// that the selected package provides the field's current resources.
    pub(crate) fn remap_projected_owner<E: TaskEncoder>(
        &mut self,
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
        root: mir::Local,
        group: usize,
        fields: &[mir::PlaceElem<'vir>],
        block: mir::BasicBlock,
    ) {
        let Some(node) = visitor
            .wands
            .inputs
            .iter()
            .copied()
            .find(|node| node.mir_local() == root && node.region_idx().index() == group)
        else {
            return;
        };
        let Some(&(pre, _)) = visitor.call_labels.get(&block) else {
            return;
        };
        let args = PledgeExpr::pledge_args(
            visitor.local_defs[mir::RETURN_PLACE].impure_snap,
            visitor.body.args_iter().map(|arg| {
                visitor
                    .vcx
                    .mk_local_labelled_old_expr(visitor.local_defs[arg].impure_snap, pre)
            }),
        );
        let Some(owner) = visitor
            .wands
            .input_owner(node, args, None, visitor.vcx, visitor.deps)
        else {
            return;
        };
        let &ty::TyKind::Ref(_, mut selected, ty::Mutability::Mut) =
            visitor.body.local_decls[root].ty.kind()
        else {
            return;
        };
        let mut path = Vec::new();
        for field in fields {
            let mir::ProjectionElem::Field(index, field_ty) = *field else {
                return;
            };
            path.push(visitor.ty_use_pure(selected).expect_structlike()[index].projection_name());
            selected = field_ty;
        }
        self.projection_path = Some(visitor.vcx.alloc_slice(&path));
        self.origin = Some(root);
        self.selection = None;
        self.group = group;
        self.owner = owner;
    }
}

struct HandoffSubstitute<'vir>(FxHashMap<&'vir str, vir::ExprDyn<'vir>>);
impl<'vir> Folder<'vir, (), !> for HandoffSubstitute<'vir> {
    fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
        if let vir::ExprKindData::Local(local) = expr.kind {
            self.0.get(local.name).copied()
        } else {
            expr.super_fold_with(self)
        }
    }
}

impl<'vir> OwnerHandoff<'vir> {
    pub(crate) fn argument(&self) -> mir::Local {
        self.argument
    }

    pub(crate) fn group(&self) -> usize {
        self.group
    }

    pub(crate) fn restore<E: TaskEncoder>(
        &self,
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
        guards: Vec<vir::ExprBool<'vir>>,
        origin: Option<mir::Local>,
        selection: Option<vir::ExprInt<'vir>>,
    ) -> RestoredOwnerHandoff<'vir> {
        let mut substitute = HandoffSubstitute(FxHashMap::default());
        for &(decl, value) in &self.bindings {
            let value = value.fold_with(&mut substitute).unwrap_or(value);
            let saved = visitor.new_tmp(value.ty());
            visitor.stmt(visitor.vcx.mk_pure_assign_stmt(saved, value));
            substitute.0.insert(decl.name, saved.as_dyn());
        }
        let current = self
            .current
            .fold_with(&mut substitute)
            .unwrap_or(self.current);
        RestoredOwnerHandoff {
            origin,
            selection,
            owner: self.owner,
            group: self.group,
            continuation: resource_restoration_wand(visitor.vcx, current, self.entry),
            projection_path: None,
            guards,
        }
    }

    fn consequence(&self, vcx: &'vir vir::VirCtxt<'vir>) -> vir::ExprBool<'vir> {
        let continuation = resource_restoration_wand(vcx, self.current, self.entry);
        let mut result = vcx.mk_conj(&[self.current, vcx.mk_wand_expr(continuation)]);
        for &(decl, value) in self.bindings.iter().rev() {
            result = vcx.mk_let_expr(decl, value, result);
        }
        result
    }

    fn package<E: TaskEncoder>(
        &self,
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
        script: &mut Vec<vir::Stmt<'vir>>,
    ) {
        let mut substitute = HandoffSubstitute(FxHashMap::default());
        for &(decl, value) in &self.bindings {
            let value = value.fold_with(&mut substitute).unwrap_or(value);
            let name = vir::vir_format!(visitor.vcx, "_handoff_tmp{}", visitor.tmp_ctr);
            visitor.tmp_ctr += 1;
            let captured_decl = visitor.vcx.mk_local_decl(name, value.ty());
            let local = visitor.vcx.mk_local_ex(captured_decl);
            script.push(visitor.vcx.mk_local_decl_stmt(captured_decl, None));
            script.push(visitor.vcx.mk_pure_assign_stmt(local, value));
            substitute.0.insert(decl.name, local.as_dyn());
        }
        let current = self
            .current
            .fold_with(&mut substitute)
            .unwrap_or(self.current);
        let vcx = visitor.vcx;
        let reservation = vcx.mk_wand(vcx.mk_bool::<true>(), current);
        script.push(vcx.mk_package_stmt(reservation, &[]));
        let restore =
            visitor.restore_owner_handoffs(self.owner, self.group, Some(self.argument), None);
        script.push(vcx.mk_package_stmt(
            resource_restoration_wand(vcx, current, self.entry),
            vcx.alloc_slice(&direct_resource_restoration(
                vcx, current, self.entry, restore,
            )),
        ));
        script.push(vcx.mk_apply_stmt(reservation));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceStage {
    Storage,
    Nested,
}

/// Encodes the magic wands given a function signature.
pub struct WandEnc;

#[derive(Clone, Debug)]
pub enum WandEncError {
    Unsupported(String),
}

impl<'vir, E: TaskEncoder> ImpureEncVisitor<'vir, '_, E> {
    fn restore_owner_handoffs(
        &self,
        owner: OwnerIdentity<'vir>,
        group: usize,
        origin: Option<mir::Local>,
        projection_path: Option<&[&str]>,
    ) -> Vec<vir::Stmt<'vir>> {
        let mut restored = self
            .restored_owner_handoffs
            .iter()
            .rev()
            .filter(|handoff| handoff.group == group)
            .filter(|handoff| {
                projection_path.is_none_or(|path| handoff.projection_path == Some(path))
            })
            .filter(|handoff| match (origin, handoff.origin) {
                (Some(left), Some(right)) => left == right,
                _ => true,
            })
            .map(|handoff| {
                let mut guards = handoff.guards.clone();
                if let (Some(selected), Some(origin)) = (handoff.selection, origin) {
                    let expected = self
                        .vcx
                        .mk_const_expr(vir::ConstData::Int(origin.index() as u128))
                        .downcast_ty::<vir::Int>();
                    let unknown = self.vcx.mk_eq_expr(selected, self.vcx.mk_uint::<0>());
                    let matches = self.vcx.mk_eq_expr(selected, expected);
                    guards.push(self.vcx.mk_disj(&[unknown, matches]));
                }
                guards.push(self.vcx.mk_eq_expr(owner.address, handoff.owner.address));
                guards.push(self.vcx.mk_eq_expr(owner.ty, handoff.owner.ty));
                self.vcx.mk_if_stmt(
                    self.vcx.mk_conj(&guards),
                    self.vcx
                        .alloc_slice(&[self.vcx.mk_apply_stmt(handoff.continuation)]),
                    &[],
                )
            })
            .collect::<Vec<_>>();
        restored.extend(
            self.owner_summaries
                .iter()
                .filter(|summary| Some(summary.origin) == origin && summary.group == group)
                .map(|summary| {
                    self.vcx.mk_apply_stmt(resource_restoration_wand(
                        self.vcx,
                        summary.current,
                        summary.entry,
                    ))
                }),
        );
        restored
    }

    /// Map a signature input to the body graph's original resource root.
    pub(crate) fn input_resource_root(
        &self,
        input: FunctionShapeInput<Generalized>,
    ) -> BlockedNode<'vir> {
        let local = input.mir_local();
        let node: FunctionShapeNode<Generalized> = input.into();
        if input.region_idx().index() == 0
            && matches!(
                node.ty(self.wands.fn_sig(self.vcx, None)).kind(),
                ty::TyKind::Ref(_, _, ty::Mutability::Mut)
            )
        {
            return mir::Place::from(local).into();
        }
        let generalized: IndexVec<_, GeneralizedLifetime<'vir>> = self
            .vcx
            .tcx()
            .extract_regions(node.ty(self.wands.fn_sig(self.vcx, None)));
        let actual: IndexVec<_, PcgRegion<'vir>> = self
            .pcg_ctxt()
            .extract_regions(self.body.local_decls[local].ty);
        // Body and signature extraction now both retain opaque resource
        // groups. Their ordered type traversal identifies corresponding roots.
        let index = input.region_idx().index();
        if index < actual.len()
            && generalized.len() == actual.len()
            && generalized
                .iter()
                .zip(actual.iter())
                .all(|(signature, body)| {
                    matches!(signature, GeneralizedLifetime::RegionsIn(_))
                        == matches!(body, PcgRegion::RegionsIn(_))
                })
        {
            LifetimeProjection::from_index(
                RemotePlace::new(local),
                input.region_idx().index().into(),
            )
            .into()
        } else {
            mir::Place::from(local).into()
        }
    }

    pub fn package_wands(
        &mut self,
        final_borrow_state: &BorrowsState<'_, 'vir>,
        already_unblocked: &[BorrowPcgEdge<'vir>],
    ) -> EncodeResult<'vir, Vec<vir::Stmt<'vir>>, E> {
        let mut wand_packages = Vec::new();
        let label = self.new_label("package_post");
        let result = self.local_defs[mir::RETURN_PLACE].impure_snap;
        let result = self.vcx.mk_local_labelled_old_expr(result, label);
        let args = self
            .local_defs
            .args()
            .map(|a| self.vcx.mk_old_expr(a.impure_snap));
        let args = PledgeExpr::pledge_args(result, args);

        let wands = self.wands.viper_wands();
        // Indexed physical reservations protect parent observations shared by
        // dependent stages. Independent restoration has no such parent frame;
        // keep its existing antecedent proof without another quantified wand.
        let dependent_stages = wands
            .iter()
            .any(|wand| !wand.frames.is_empty() || wand.bundled_owner);
        // Immediate opaque and dependent indexed resources must survive packaging.
        // Hide them in a checked temporary wand so a restoration script can
        // only consume the matching permissions from its own antecedent.
        // Their resource keys are frozen at the package's entry heap. Indexed
        // parent predicates also supply the shallow sequence snapshots bound
        // outside nested restoration wands. Preserve their actual heap state
        // while constructing those wands from their own antecedents.
        let retained_bindings = self.vcx.alloc(std::cell::RefCell::new(Vec::new()));
        let retained = if wands.is_empty() {
            Vec::new()
        } else {
            self.wands
                .output_resource_nodes(self.vcx)
                .into_iter()
                .filter_map(|node| {
                    let regions: IndexVec<_, GeneralizedLifetime<'vir>> = self
                        .vcx
                        .tcx()
                        .extract_regions(node.ty(self.wands.fn_sig(self.vcx, None)));
                    let opaque = matches!(
                        regions.get(node.region_idx()),
                        Some(GeneralizedLifetime::RegionsIn(_))
                    );
                    let expression = self.wands.encode_predicates_for_function_shape_node(
                        self.vcx,
                        self.deps,
                        node,
                        None,
                        Some(vir::OldLabel::Label(label)),
                        Some(retained_bindings),
                        |i| args[i],
                    )?;
                    struct HasIndexed(bool);
                    impl<'vir> Folder<'vir, (), !> for HasIndexed {
                        fn fold_expr(
                            &mut self,
                            e: vir::ExprDyn<'vir>,
                        ) -> Option<vir::ExprDyn<'vir>> {
                            if let vir::ExprKindData::PredicateApp(app) = e.kind
                                && app.target.starts_with("indexed_")
                            {
                                self.0 = true;
                            }
                            e.super_fold_with(self)
                        }
                    }
                    let mut indexed = HasIndexed(false);
                    let _ = expression.fold_with(&mut indexed);
                    (opaque || (dependent_stages && indexed.0)).then_some(expression)
                })
                .collect::<Vec<_>>()
        };
        let retained = (!retained.is_empty()).then(|| {
            self.vcx
                .mk_wand(self.vcx.mk_bool::<true>(), self.vcx.mk_conj(&retained))
        });
        // Use the same saved resource keys when packaging and applying the
        // reservation. Re-evaluating a heap-dependent observer after other
        // packages can otherwise give Silicon different wand arguments.
        let retained = retained.map(|wand| {
            struct Substitute<'vir>(FxHashMap<&'vir str, vir::ExprDyn<'vir>>);
            impl<'vir> Folder<'vir, (), !> for Substitute<'vir> {
                fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                    if let vir::ExprKindData::Local(local) = e.kind {
                        self.0.get(local.name).copied()
                    } else {
                        e.super_fold_with(self)
                    }
                }
            }
            let mut substitute = Substitute(FxHashMap::default());
            for &(decl, value) in retained_bindings.borrow().iter() {
                let value = value.fold_with(&mut substitute).unwrap_or(value);
                let local = self.new_tmp(value.ty());
                wand_packages.push(self.vcx.mk_pure_assign_stmt(local, value));
                substitute.0.insert(decl.name, local.as_dyn());
            }
            wand.fold_with(&mut substitute).unwrap_or(wand)
        });
        if let Some(retained) = retained {
            wand_packages.push(self.vcx.mk_package_stmt(retained, &[]));
        }
        // A callee may have one restoration obligation covering several
        // instantiated groups. Output packages consume that linear obligation
        // once; subsequent packages must use resources supplied by their own
        // checked antecedents, not replay the same callee wand.
        let mut consumed_call_wands = Vec::new();
        for wand_data in wands {
            let outer_handoffs = self.restored_owner_handoffs.len();
            let ctxt = self.pcg_ctxt();
            let opaque_action = |edge: &BorrowPcgEdge<'vir>| match edge.kind() {
                BorrowPcgEdgeKind::BorrowPcgExpansion(expansion) => {
                    matches!(expansion.base(), pcg::pcg::PcgNode::LifetimeProjection(lp) if matches!(lp.region(ctxt), PcgRegion::RegionsIn(_)))
                }
                BorrowPcgEdgeKind::BorrowFlow(flow) => {
                    matches!(flow.long().region(ctxt), PcgRegion::RegionsIn(_))
                }
                _ => false,
            };
            let sig = self.wands.fn_sig(self.vcx, None);
            let restores_opaque = wand_data.rhs.iter().any(|node| {
                let node: FunctionShapeNode<Generalized> = (*node).into();
                let regions: IndexVec<_, GeneralizedLifetime<'vir>> =
                    self.vcx.tcx().extract_regions(node.ty(sig));
                matches!(
                    regions.get(node.region_idx()),
                    Some(GeneralizedLifetime::RegionsIn(_))
                )
            });
            let mut prior_edges = already_unblocked.to_vec();
            // The antecedent supplies these already restored permissions.
            // Replaying their physical folds would consume them a second time.
            for frame in &wand_data.frames {
                let root = self.input_resource_root(*frame);
                let graph = UnblockGraph::for_node(root, final_borrow_state, self.pcg_ctxt());
                prior_edges.extend(
                    graph
                        .actions(self.pcg_ctxt())
                        .unwrap()
                        .iter()
                        .filter(|a| !opaque_action(a.edge())
                            && !matches!(a.edge().kind(), BorrowPcgEdgeKind::Coupled(
                                pcg::coupling::PcgCoupledEdgeKind(
                                    pcg::borrow_pcg::edge::abstraction::FunctionCallOrLoop::FunctionCall(_)
                                )
                            )))
                        .map(|a| a.edge().clone()),
                );
            }
            let bindings = self.vcx.alloc(std::cell::RefCell::new(Vec::new()));
            let Some(mut wand) = self.wands.mk_wand(
                &wand_data,
                args,
                None,
                Some(label),
                Some(bindings),
                None,
                &[],
                self.vcx,
                self.deps,
            ) else {
                continue;
            };
            // Match the contract's free post-call snapshots with explicit
            // values outside the package. In particular, a captured sequence
            // must stay outside its element quantifier on both sides of the
            // method boundary; leaving its heap read inside an indexed term
            // can give Viper a different wand shape.
            struct Substitute<'vir>(FxHashMap<&'vir str, vir::ExprDyn<'vir>>);
            impl<'vir> Folder<'vir, (), !> for Substitute<'vir> {
                fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                    if let vir::ExprKindData::Local(local) = e.kind {
                        self.0.get(local.name).copied()
                    } else {
                        e.super_fold_with(self)
                    }
                }
            }
            let mut substitute = Substitute(FxHashMap::default());
            for &(decl, value) in bindings.borrow().iter() {
                let value = value.fold_with(&mut substitute).unwrap_or(value);
                let local = self.new_tmp(value.ty());
                wand_packages.push(self.vcx.mk_pure_assign_stmt(local, value));
                substitute.0.insert(decl.name, local.as_dyn());
            }
            wand = wand.fold_with(&mut substitute).unwrap_or(wand);
            let rhs = wand_data
                .rhs
                .iter()
                .map(|g| {
                    let resources = self.wands.encode_predicates_for_function_shape_node(
                        self.vcx,
                        self.deps,
                        *g,
                        None,
                        Some(vir::OldLabel::None),
                        None,
                        |i| args[i],
                    );
                    let root = self.input_resource_root(*g);
                    (root, resources.into_iter().collect())
                })
                .collect();
            let stage = self
                .wands
                .resource_stage(&wand_data, None, self.vcx, self.deps);
            let previous_stage = std::mem::replace(&mut self.package_resource_stage, stage);
            let package_script = self.pcs_package_script_with_consumed_wands(
                final_borrow_state,
                rhs,
                label,
                &prior_edges,
                &mut consumed_call_wands,
                |edge| restores_opaque || !opaque_action(edge),
            );
            self.package_resource_stage = previous_stage;
            let mut package_script = package_script?;

            let mut owner_restorations = Vec::new();
            for &node in &wand_data.rhs {
                if node.region_idx().index() != 0
                    && !self.wands.is_handoff_node(node, &wand_data, self.vcx)
                {
                    if let Some(owner) = self
                        .wands
                        .input_owner(node, args, None, self.vcx, self.deps)
                    {
                        let history = self.restore_owner_handoffs(
                            owner,
                            node.region_idx().index(),
                            Some(node.mir_local()),
                            None,
                        );
                        let mut observe = |label| {
                            self.wands.encode_predicates_for_function_shape_node(
                                self.vcx,
                                self.deps,
                                node,
                                None,
                                Some(label),
                                None,
                                |i| args[i],
                            )
                        };
                        let current = observe(vir::OldLabel::Label(label))
                            .unwrap_or_else(|| self.vcx.mk_bool::<true>());
                        let entry = observe(vir::OldLabel::None)
                            .unwrap_or_else(|| self.vcx.mk_bool::<true>());
                        let mut paths = self
                            .restored_owner_handoffs
                            .iter()
                            .filter(|handoff| {
                                handoff.group == node.region_idx().index()
                                    && handoff.origin == Some(node.mir_local())
                            })
                            .filter_map(|handoff| handoff.projection_path)
                            .collect::<Vec<_>>();
                        paths.sort();
                        paths.dedup();
                        // A whole-owner continuation or loop summary cannot be
                        // split into independent projected-field histories.
                        if self.restored_owner_handoffs.iter().any(|handoff| {
                            handoff.group == node.region_idx().index()
                                && handoff.origin == Some(node.mir_local())
                                && handoff.projection_path.is_none()
                        }) || self.owner_summaries.iter().any(|summary| {
                            summary.group == node.region_idx().index()
                                && summary.origin == node.mir_local()
                        }) {
                            paths.clear();
                        }
                        let split = projected_resource_parts(current, entry, &paths);
                        if let Some(parts) = split {
                            for (path, current, entry) in parts {
                                let history = self.restore_owner_handoffs(
                                    owner,
                                    node.region_idx().index(),
                                    Some(node.mir_local()),
                                    Some(path),
                                );
                                owner_restorations.push((current, entry, history));
                            }
                        } else {
                            owner_restorations.push((current, entry, history));
                        }
                    }
                }
            }
            let mut restore_entry = ordered_owner_restoration(self.vcx, owner_restorations);
            restore_entry.extend(package_script);
            let mut package_script = restore_entry;
            for &node in &wand_data.rhs {
                if let Some(entry) = self.wands.encode_predicates_for_function_shape_node(
                    self.vcx,
                    self.deps,
                    node,
                    None,
                    Some(vir::OldLabel::None),
                    None,
                    |i| args[i],
                ) {
                    if let Some(handoff) = self
                        .wands
                        .owner_handoff(node, &wand_data, args, None, entry, self.vcx, self.deps)
                    {
                        handoff.package(self, &mut package_script);
                    }
                }
            }

            if let Some(composed) = crate::paired_resources::compose_flat_restoration(
                self.vcx,
                wand,
                &package_script,
                &mut self.tmp_ctr,
            ) {
                package_script = composed;
            }

            package_script =
                direct_resource_restoration(self.vcx, wand.lhs, wand.rhs, package_script);

            package_script.extend(crate::encoders::ty::indexed_coherence::restoration_calls(
                self.vcx, wand.lhs, wand.rhs,
            ));

            for EncodedPledge {
                expiry_postcondition,
                ..
            } in &wand_data.pledges
            {
                let span = expiry_postcondition.span();
                self.vcx.with_span(span, |vcx| {
                    vcx.handle_error("exhale.failed:assertion.false", move |_| {
                        Some(vec![PrustiError::verification(
                            "pledge postcondition might not hold",
                            span.into(),
                        )])
                    });
                    package_script.push(vcx.mk_exhale_stmt(expiry_postcondition.expr(
                        args,
                        Some(PostCapture {
                            freeze_current: false,
                            label: Some(vir::OldLabel::Label(label)),
                            bindings: None,
                            memo: None,
                        }),
                    )));
                });
            }
            wand_packages.push(
                self.vcx
                    .mk_package_stmt(wand, self.vcx.alloc_slice(&package_script)),
            );
            self.restored_owner_handoffs.truncate(outer_handoffs);
        }
        if let Some(retained) = retained {
            wand_packages.push(self.vcx.mk_apply_stmt(retained));
        }
        Ok(wand_packages)
    }
}

type EncodedPledges<'vir> = Vec<EncodedPledge<'vir>>;

/// Not tied to a caller or callee context. `indirect_pres`, `indirect_posts`,
/// `wand_posts`, and `package_wands` are identity-substituted and intended for
/// use in the callee's own contract; `call_wands` is for caller use and
/// re-substitutes via a [`WandCallContext`].
#[derive(Clone)]
pub struct WandEncOutput<'vir> {
    /// Information about the corresponding function.
    function_data: FunctionData<'vir>,

    /// The lifetime projections of all arguments to the function.
    inputs: Vec<FunctionShapeInput<Generalized>>,

    /// The lifetime projections of all function outputs (according to the
    /// corresponding [`FunctionShape`]). This *includes* lifetime projections
    /// of nested lifetimes in the function arguments.
    outputs: Vec<FunctionShapeOutput<Generalized>>,

    /// Encoded VIR expressions for the magic wands.
    wands: Vec<WandData<'vir>>,
}

/// Substitution context for instantiating a wand at a call site. When `None`,
/// the wand is encoded using the callee's identity substitution (appropriate
/// when emitting wands inside the function being defined). When `Some`, the
/// wand is re-encoded with the call-site substitutions and the caller's
/// generic parameters, so that placeholders like `Self` or other callee
/// generics are replaced by concrete types from the caller's perspective.
pub type WandCallContext<'vir> = Option<GArgs<'vir>>;

impl<'vir> WandEncOutput<'vir> {
    /// Classify a stage by the resource-bearing inputs it consumes. Mixed
    /// stages retain the full checked script; they cannot be split here.
    pub(crate) fn resource_stage<E: TaskEncoder>(
        &self,
        wand: &WandData<'vir>,
        call: Option<(&[ty::Ty<'vir>], ty::Ty<'vir>)>,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, E>,
    ) -> Option<ResourceStage> {
        let mapping = if let Some((arguments, result)) = call {
            Some(
                self.function_data
                    .call_projection_map(arguments, result, vcx.tcx())
                    .ok()?,
            )
        } else {
            None
        };
        let mut stage = None;
        for node in &wand.lhs {
            let Some(predicates) = self.predicates_for_function_shape_node(vcx, deps, *node, None)
            else {
                continue;
            };
            // Instantiation can turn an opaque T into an outer &mut. Use the
            // call's actual projections, not its definition's generic slots.
            let projections = if let (Some(mapping), Some((arguments, result))) = (&mapping, call) {
                mapping
                    .call_nodes(*node)?
                    .iter()
                    .map(|mapped| {
                        let ty = if mapped.mir_local() == mir::RETURN_PLACE {
                            result
                        } else {
                            arguments[mapped.mir_local().index() - 1]
                        };
                        (ty, mapped.region_idx().index())
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![(node.ty(self.fn_sig(vcx, None)), node.region_idx().index())]
            };
            // Aggregate outputs can contain several independently returned
            // references. Their outer storage permissions are still a storage
            // stage even though the aggregate itself is not a reference. Only
            // classify a wholly direct physical footprint this way: a heap
            // read in a predicate argument or an indexed/opaque predicate keeps
            // the group in the nested stage.
            let aggregate_storage = if matches!(
                node.ty(self.fn_sig(vcx, None)).kind(),
                ty::TyKind::Tuple(_) | ty::TyKind::Adt(..)
            ) {
                use vir::Reify;
                struct DirectStorage {
                    physical: bool,
                    nested: bool,
                }
                impl<'vir> Folder<'vir, (), !> for DirectStorage {
                    fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                        match e.kind {
                            vir::ExprKindData::PredicateApp(app) if app.target == "p_Param" => {
                                self.physical = true;
                            }
                            vir::ExprKindData::PredicateApp(_)
                            | vir::ExprKindData::Field(..)
                            | vir::ExprKindData::Unfolding(_) => self.nested = true,
                            vir::ExprKindData::FuncApp(app)
                                if app.target.starts_with("p_")
                                    || app.target.starts_with("indexed_")
                                    || app.target.starts_with("indirect_") =>
                            {
                                self.nested = true
                            }
                            _ => {}
                        }
                        e.super_fold_with(self)
                    }
                }
                let decomposition = RustTyDecomposition::from_ty(
                    node.ty(self.fn_sig(vcx, None)),
                    self.g_params(vcx, None),
                );
                let domain = deps
                    .require_dep::<crate::encoders::TyUsePureEnc>(decomposition)
                    .ok()?;
                let snapshot =
                    vcx.mk_local_ex(vcx.mk_local_decl("_stage_argument", domain.snapshot));
                let mut footprint = DirectStorage {
                    physical: false,
                    nested: false,
                };
                for predicate in &predicates {
                    let expression = predicate.reify(vcx, ExprInput::current(snapshot));
                    let _ = expression.fold_with(&mut footprint);
                }
                footprint.physical && !footprint.nested
            } else {
                false
            };
            for (ty, index) in projections {
                let storage = aggregate_storage
                    || index == 0
                        && matches!(ty.kind(), ty::TyKind::Ref(_, _, ty::Mutability::Mut));
                // One deduplicated lifetime can cover both storage and inner
                // references. Such a group must retain its complete script.
                if storage && !aggregate_storage && predicates.len() > 1 {
                    return None;
                }
                let current = if storage {
                    ResourceStage::Storage
                } else {
                    ResourceStage::Nested
                };
                if stage.is_some_and(|previous| previous != current) {
                    return None;
                }
                stage = Some(current);
            }
        }
        stage
    }

    pub(crate) fn fn_sig(
        &self,
        vcx: &'vir vir::VirCtxt<'vir>,
        call_ctx: WandCallContext<'vir>,
    ) -> ty::FnSig<'vir> {
        match call_ctx {
            // TODO: change pcg's `fn_sig` to take `&[GenericArg]` instead of `GenericArgsRef`
            Some(ctx) => self
                .function_data
                .fn_sig(vcx.tcx(), vcx.tcx().mk_args(ctx.args())),
            None => self.function_data.identity_fn_sig(vcx.tcx()),
        }
    }

    pub(crate) fn g_params(
        &self,
        vcx: &'vir vir::VirCtxt<'vir>,
        call_ctx: WandCallContext<'vir>,
    ) -> GParams<'vir> {
        match call_ctx {
            Some(ctx) => ctx.context(),
            None => GParams::new(
                self.function_data.identity_substs(vcx.tcx()),
                self.function_data.param_env(vcx.tcx()),
                false,
            ),
        }
    }

    /// The (unreified) predicates associated with the given node, or `None` if
    /// there are no resources associated with it.
    #[allow(clippy::type_complexity)]
    pub(crate) fn predicates_for_function_shape_node(
        &self,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, impl TaskEncoder>,
        g: FunctionShapeNode<Generalized>,
        call_ctx: WandCallContext<'vir>,
    ) -> Option<Vec<vir::ExprGenBool<'vir, ExprInput<'vir>, vir::ExprKind<'vir>>>> {
        // Preserve the definition-site interface. Instantiating the Rust type
        // first can turn one whole group into several concrete lifetime slots.
        let _ = call_ctx;
        let arg_ty = g.ty(self.fn_sig(vcx, None));
        let decomp = RustTyDecomposition::from_ty(arg_ty, self.g_params(vcx, None));
        let region_proj =
            projection_for_generalized_idx(arg_ty, g.region_idx(), decomp, vcx.tcx())?;
        let predicates = deps
            .require_dep::<IndirectPredicatesEnc>(region_proj)
            .unwrap()
            .predicate_applications;
        (!predicates.is_empty()).then_some(predicates)
    }

    fn encode_predicates_for_function_shape_node(
        &self,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, impl TaskEncoder>,
        g: impl Into<FunctionShapeNode<Generalized>>,
        call_ctx: WandCallContext<'vir>,
        heap_label: Option<vir::OldLabel<'vir>>,
        bindings: Option<&'vir SnapshotBindings<'vir>>,
        mut snap: impl FnMut(mir::Local) -> vir::ExprSnap<'vir>,
    ) -> Option<vir::ExprBool<'vir>> {
        use vir::Reify;
        let g = g.into();
        let predicates = self.predicates_for_function_shape_node(vcx, deps, g, call_ctx)?;

        let local = g.mir_local();
        let mut local_snap = snap(local);
        if let Some(args) = call_ctx {
            let signature = RustSignature::new(self.function_data.def_id());
            let source = if local == mir::RETURN_PLACE {
                signature.output
            } else {
                signature.inputs[local.index() - 1]
            };
            let caster = deps
                .require_dep::<GArgsCastEnc<Pure>>(
                    source.decompose_compare_normalize(signature.gparams, args),
                )
                .unwrap();
            local_snap = caster.cast_to_callee_ctx(local_snap);
        }
        let substitutions = self.generics_subst(call_ctx, vcx, deps);
        // Build the definition-site expression before inserting a caller
        // snapshot. Otherwise a caller's type local can be substituted twice
        // when it has the same name as a callee parameter.
        let placeholder = vcx.mk_local_decl("_resource_argument", local_snap.ty());
        let binding_start = bindings.map(|bindings| bindings.borrow().len());
        let expression = vcx.mk_conj(
            &predicates
                .iter()
                .map(|p| {
                    p.reify(
                        vcx,
                        ExprInput {
                            snap: vcx.mk_local_ex(placeholder),
                            heap_label,
                            bindings,
                            guard: None,
                            extent: None,
                            metadata: None,
                            binder_depth: 0,
                            indexed: None,
                            indexed_snapshots: &[],
                            resource_path: &[],
                        },
                    )
                })
                .collect::<Vec<_>>(),
        );
        fn instantiate<'vir, T: vir::CompType>(
            vcx: &'vir vir::VirCtxt<'vir>,
            expression: vir::Expr<'vir, T>,
            substitutions: &'vir FxHashMap<&'vir str, vir::ExprDyn<'vir>>,
            snapshot: vir::ExprSnap<'vir>,
        ) -> vir::Expr<'vir, T> {
            use vir::{Foldable, Folder, Reify};
            struct Insert<'vir>(vir::ExprSnap<'vir>);
            impl<'vir> Folder<'vir, (), !> for Insert<'vir> {
                fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                    if matches!(e.kind, vir::ExprKindData::Local(local) if local.name == "_resource_argument")
                    {
                        Some(self.0.as_dyn())
                    } else {
                        e.super_fold_with(self)
                    }
                }
            }
            let expression =
                vcx.with_local_subst(substitutions, || expression.lift().reify(vcx, ()));
            expression
                .fold_with(&mut Insert(snapshot))
                .unwrap_or(expression)
        }
        if let (Some(bindings), Some(start)) = (bindings, binding_start) {
            for (_, value) in &mut bindings.borrow_mut()[start..] {
                *value = instantiate(vcx, *value, substitutions, local_snap);
            }
        }
        Some(instantiate(vcx, expression, substitutions, local_snap))
    }

    pub fn indirect_pres<'a, E: TaskEncoder>(
        &'a self,
        vcx: &'vir vir::VirCtxt<'vir>,
        local_defs: &'a MirLocalDefEncOutput<'vir>,
        deps: &'a mut TaskEncoderDependencies<'vir, E>,
    ) -> impl Iterator<Item = vir::ExprBool<'vir>> + 'a {
        self.inputs().filter_map(|g| {
            self.encode_predicates_for_function_shape_node(vcx, deps, g, None, None, None, |i| {
                local_defs[i].impure_snap
            })
        })
    }

    pub fn indirect_posts<'a, E: TaskEncoder>(
        &'a self,
        vcx: &'vir vir::VirCtxt<'vir>,
        local_defs: &'a MirLocalDefEncOutput<'vir>,
        deps: &'a mut TaskEncoderDependencies<'vir, E>,
    ) -> impl Iterator<Item = vir::ExprBool<'vir>> + 'a {
        // The encoded predicates for the input lifetime projections that are
        // not blocked by any of the result lifetime projections. These will be
        // encoded as part of the postcondition of the function (in contrast,
        // the predicates for the blocked inputs will appear on the right-hand
        // side of a magic wand in the postcondition).
        let unblocked_input_posts = self
            .inputs()
            .filter(|i| !self.blocked_inputs().contains(i))
            .filter_map(|lp| {
                self.encode_predicates_for_function_shape_node(
                    vcx,
                    deps,
                    lp,
                    None,
                    Some(vir::OldLabel::None),
                    None,
                    |i| vcx.mk_old_expr(local_defs[i].impure_snap),
                )
            })
            .collect::<Vec<_>>()
            .into_iter();

        // An invariant lifetime nested in a mutable argument can remain in
        // that argument after the call while another part is returned. Give
        // the caller that post-call resource immediately; the wand later
        // reassembles it with the returned resources.
        let output_nodes = self.output_resource_nodes(vcx);
        let output_posts = output_nodes.into_iter().filter_map(|g| {
            self.encode_predicates_for_function_shape_node(vcx, deps, g, None, None, None, |i| {
                if i == mir::RETURN_PLACE {
                    local_defs[i].impure_snap
                } else {
                    vcx.mk_old_expr(local_defs[i].impure_snap)
                }
            })
        });
        unblocked_input_posts.chain(output_posts)
    }

    pub(crate) fn output_resource_nodes(
        &self,
        vcx: &'vir vir::VirCtxt<'vir>,
    ) -> Vec<FunctionShapeNode<Generalized>> {
        let mut output_nodes = self.outputs().collect::<Vec<_>>();
        for node in self.wands.iter().flat_map(|wand| wand.lhs.iter().copied()) {
            if node.mir_local() != mir::RETURN_PLACE
                && matches!(
                    node.ty(self.fn_sig(vcx, None)).kind(),
                    ty::TyKind::Ref(_, _, ty::Mutability::Mut)
                )
                && !output_nodes.contains(&node)
            {
                output_nodes.push(node);
            }
        }
        // Later projections can observe snapshots inside earlier physical
        // borrows. In particular, a retained iterator tail must be returned
        // before its element-resource quantifier is checked for well-formedness.
        output_nodes.sort_by_key(|node| (node.mir_local(), node.region_idx()));
        output_nodes
    }

    pub fn wand_posts<'a, E: TaskEncoder>(
        &'a self,
        vcx: &'vir vir::VirCtxt<'vir>,
        local_defs: &'a MirLocalDefEncOutput<'vir>,
        deps: &'a mut TaskEncoderDependencies<'vir, E>,
    ) -> impl Iterator<Item = vir::ExprBool<'vir>> + 'a {
        let wand_result =
            vcx.mk_local_decl("wand_result", local_defs[mir::RETURN_PLACE].local_snap.ty());
        let wand_result_expr = vcx.mk_local_ex(wand_result);
        let args = local_defs
            .args()
            .map(|arg| vcx.mk_old_expr(arg.impure_snap));
        let args = PledgeExpr::pledge_args(wand_result_expr, args);

        // TODO: wands for late-bound regions
        self.viper_wands().into_iter().filter_map(move |wand_data| {
            // A wand's heap-dependent expressions are interpreted when it
            // is applied. Capture the argument's post-call snapshots outside
            // the wand so later changes or moves cannot change its shape.
            let bindings = vcx.alloc(std::cell::RefCell::new(Vec::new()));
            let wand = self.mk_wand(
                &wand_data,
                args,
                None,
                None,
                Some(bindings),
                None,
                &[],
                vcx,
                deps,
            )?;
            let mut wand = vcx.mk_wand_expr(wand);
            for &(decl, value) in bindings.borrow().iter().rev() {
                wand = vcx.mk_let_expr(decl, value, wand);
            }
            Some(vcx.mk_let_expr(wand_result, local_defs[mir::RETURN_PLACE].impure_snap, wand))
        })
    }

    pub fn call_wands<E: TaskEncoder>(
        &self,
        arguments: &[vir::ExprSnap<'vir>],
        label_pre: &'vir str,
        label_post: &'vir str,
        call_ctx: GArgs<'vir>,
        extra_pledges: &[PledgeExpr<'vir>],
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
    ) -> EncodeResult<'vir, Vec<vir::Wand<'vir>>, E> {
        Ok(self
            .selected_call_wands(
                arguments,
                label_pre,
                label_post,
                call_ctx,
                extra_pledges,
                None,
                false,
                visitor,
            )?
            .into_iter()
            .map(|(_, wand, _, _, _, _)| wand)
            .collect())
    }

    /// The index identifies an obligation in this function's retained wand
    /// list, before substitution or omission of empty instantiated wands.
    /// `materialize` is used at the call site: the caller must retain the
    /// returned wands so expiry and loop invariants use those same captures.
    pub(crate) fn selected_call_wands<E: TaskEncoder>(
        &self,
        arguments: &[vir::ExprSnap<'vir>],
        label_pre: &'vir str,
        label_post: &'vir str,
        call_ctx: GArgs<'vir>,
        extra_pledges: &[PledgeExpr<'vir>],
        selected: Option<&[usize]>,
        materialize: bool,
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
    ) -> EncodeResult<'vir, Vec<CallWand<'vir>>, E> {
        if self.wands.is_empty() {
            return Ok(Vec::new());
        }
        let vcx = visitor.vcx;
        let mut captures = CallSnapshots::default();
        let mut capture = |value: vir::ExprSnap<'vir>| {
            if materialize {
                captures.capture(value.as_dyn(), visitor).downcast_ty()
            } else {
                value
            }
        };
        let result =
            vcx.mk_local_labelled_old_expr(arguments[mir::RETURN_PLACE.as_usize()], label_post);
        let result = capture(result);
        let args = (1..arguments.len())
            .map(|l| capture(vcx.mk_local_labelled_old_expr(arguments[l], label_pre)))
            .collect::<Vec<_>>();
        let args = PledgeExpr::pledge_args(result, args);
        let mut wands = Vec::new();
        for (id, wand_data) in self.wands.iter().enumerate() {
            if selected.is_some_and(|ids| !ids.contains(&id)) {
                continue;
            }
            let bindings =
                materialize.then(|| &*visitor.vcx.alloc(std::cell::RefCell::new(Vec::new())));
            let Some(mut wand) = self.mk_wand(
                wand_data,
                args,
                Some(label_pre),
                Some(label_post),
                bindings,
                Some(call_ctx),
                extra_pledges,
                visitor.vcx,
                visitor.deps,
            ) else {
                continue;
            };
            if let Some(bindings) = bindings {
                let resources = wand_data
                    .lhs
                    .iter()
                    .filter(|node| node.mir_local() == mir::RETURN_PLACE)
                    .filter_map(|node| {
                        self.encode_predicates_for_function_shape_node(
                            vcx,
                            visitor.deps,
                            *node,
                            Some(call_ctx),
                            Some(vir::OldLabel::Label(label_post)),
                            Some(bindings),
                            |i| args[i],
                        )
                    })
                    .collect::<Vec<_>>();
                // Capture in dependency order. These are the same post-call
                // observations bound outside the callee's returned wand; no
                // read is moved out of its quantifier or expiry heap.
                let mut substitute = captures.bind(bindings, visitor);
                wand = wand.fold_with(&mut substitute).unwrap_or(wand);
                let resources = resources
                    .into_iter()
                    .map(|resource| resource.fold_with(&mut substitute).unwrap_or(resource))
                    .collect();
                visitor
                    .call_result_resources
                    .insert((visitor.current_block.unwrap(), id), resources);
            }
            let signature = RustSignature::new(self.function_data.def_id());
            let mut scalar_views = Vec::new();
            let mut convert = |node: FunctionShapeNode<Generalized>,
                               pack,
                               heap|
             -> EncodeResult<'vir, Vec<vir::Stmt<'vir>>, E> {
                let source = if node.mir_local() == mir::RETURN_PLACE {
                    signature.output
                } else {
                    signature.inputs[node.mir_local().index() - 1]
                };
                let regions: IndexVec<_, GeneralizedLifetime<'vir>> = visitor
                    .vcx
                    .tcx()
                    .extract_regions(node.ty(self.fn_sig(visitor.vcx, None)));
                if let Some(GeneralizedLifetime::RegionsIn(opaque)) = regions.get(node.region_idx())
                {
                    let conversion =
                        visitor
                            .deps
                            .require_dep::<ResourceBoundaryEnc>(ResourceBoundaryTask {
                                definition: source.decompose(signature.gparams),
                                actual: source.decompose_normalize(call_ctx),
                                selected: Some(opaque.ty(visitor.vcx.tcx())),
                            })?;
                    if pack && let Some(label) = heap {
                        scalar_views
                            .extend(conversion.scalar_views_on_heap(args[node.mir_local()], label));
                    }
                    Ok(match heap {
                        Some(label) => {
                            conversion.at_snapshot_on_heap(args[node.mir_local()], pack, label)
                        }
                        None => conversion.at_snapshot(args[node.mir_local()], pack),
                    })
                } else {
                    Ok(Vec::new())
                }
            };
            let mut before = Vec::new();
            for node in &wand_data.lhs {
                before.extend(convert(*node, true, Some(label_post))?);
            }
            // A preceding stage returned these resources in the caller's
            // representation. Repack the frame on its original input heap
            // before passing it through the next generic restoration stage.
            for node in &wand_data.frames {
                before.extend(convert((*node).into(), true, Some(label_pre))?);
            }
            let mut after = Vec::new();
            for node in &wand_data.rhs {
                let heap = (!self.is_handoff_node(*node, wand_data, vcx)).then_some(label_pre);
                after.extend(convert((*node).into(), false, heap)?);
            }
            for node in &wand_data.frames {
                after.extend(convert((*node).into(), false, Some(label_pre))?);
            }
            // A source restored by an earlier stage may have been opened back
            // into its concrete storage predicate before this stage expires.
            // Its frame is expressed in the callee's generic representation.
            // Convert the held physical predicate for this application and
            // restore the representation expected by the surrounding graph.
            let mut storage_before = Vec::new();
            let mut storage_after = Vec::new();
            for &frame in &wand_data.frames {
                if frame.region_idx().index() != 0 {
                    continue;
                }
                let source = signature.inputs[frame.mir_local().index() - 1];
                let actual = source.decompose_normalize(call_ctx);
                let crate::encoders::ty::TySpecifics::MutRef(data) = actual.ty.specifics else {
                    continue;
                };
                // The reference constructor has a generic referent even when
                // this call instantiates it with concrete owner storage.
                let referent = data.referent.decompose_normalize(actual.args);
                if referent.ty.specifics.is_param() {
                    continue;
                }
                let Some(owner) = self.input_owner(frame, args, Some(call_ctx), vcx, visitor.deps)
                else {
                    continue;
                };
                let physical = visitor
                    .deps
                    .require_dep::<crate::encoders::TyUseImpureEnc>(referent)?;
                let caster = visitor
                    .deps
                    .require_dep::<GArgsCastEnc<crate::encoders::Impure>>(Some(
                        crate::encoders::ty::RustTyNormalized {
                            param: RustTyDecomposition::param(),
                            concrete: referent,
                        },
                    ))?;
                let concrete =
                    vcx.mk_predicate_perm_expr(physical.ref_to_pred_app(owner.address, None));
                let held = vcx
                    .mk_bin_op_expr(vir::BinOpKind::CmpLt, vcx.mk_no_perm(), concrete)
                    .downcast_ty();
                // Applications inside a package may assign only script-local
                // variables. Declare this representation flag with the
                // application, rather than with the call's retained captures.
                let flag = visitor.new_tmp_decl(vir::TYPE_BOOL);
                let was_concrete = vcx.mk_local_ex(flag);
                storage_before.push(vcx.mk_local_decl_stmt(flag, None));
                storage_before.push(vcx.mk_pure_assign_stmt(was_concrete, held));
                storage_before.push(
                    vcx.mk_if_stmt(
                        was_concrete,
                        vcx.alloc_slice(
                            &caster
                                .cast_to_callee_ctx(owner.address)
                                .into_iter()
                                .collect::<Vec<_>>(),
                        ),
                        &[],
                    ),
                );
                storage_after.push(
                    vcx.mk_if_stmt(
                        was_concrete,
                        vcx.alloc_slice(
                            &caster
                                .cast_to_caller_ctx(owner.address)
                                .into_iter()
                                .collect::<Vec<_>>(),
                        ),
                        &[],
                    ),
                );
            }
            storage_before.extend(before);
            before = storage_before;
            after.extend(storage_after);
            let mut handoffs = Vec::new();
            for &node in &wand_data.rhs {
                if let Some(entry) = self.encode_predicates_for_function_shape_node(
                    vcx,
                    visitor.deps,
                    node,
                    Some(call_ctx),
                    Some(vir::OldLabel::Label(label_pre)),
                    None,
                    |i| args[i],
                ) {
                    if let Some(handoff) = self.owner_handoff(
                        node,
                        wand_data,
                        args,
                        Some(call_ctx),
                        entry,
                        vcx,
                        visitor.deps,
                    ) {
                        handoffs.push(handoff);
                    }
                }
            }
            wands.push((id, wand, before, after, handoffs, scalar_views));
        }
        if materialize {
            let def_id = self.function_data.def_id();
            visitor
                .call_result_snapshots
                .insert(visitor.current_block.unwrap(), args[mir::RETURN_PLACE]);
            let signature = RustSignature::new(def_id);
            for (index, source) in signature.inputs.iter().enumerate() {
                let task = source.decompose_normalize(call_ctx);
                let crate::encoders::ty::TySpecifics::MutRef(data) = task.ty.specifics else {
                    continue;
                };
                let domain = visitor
                    .deps
                    .require_dep::<crate::encoders::TyUsePureEnc>(task)?;
                let inner = data.referent.decompose_context(task.ty.params, task.args);
                let inner = visitor
                    .deps
                    .require_dep::<crate::encoders::TyUseImpureEnc>(inner)?;
                let address = domain
                    .expect_mutref()
                    .deref_access(args[(index + 1).into()].downcast_ty());
                for (label, entry) in [(label_pre, true), (label_post, false)] {
                    let read = vcx.mk_local_labelled_old_expr(inner.ref_to_snap(address), label);
                    if let Some((_, captured)) = captures
                        .values
                        .iter()
                        .find(|(value, _)| CallSnapshots::same(*value, read.as_dyn()))
                    {
                        let caster = visitor.deps.require_dep::<GArgsCastEnc<Pure>>(
                            data.referent
                                .decompose_compare_normalize(task.ty.params, task.args),
                        )?;
                        let captured = caster.cast_to_caller_ctx(captured.downcast_ty());
                        let snapshots = if entry {
                            &mut visitor.call_referent_entry_snapshots
                        } else {
                            &mut visitor.call_referent_snapshots
                        };
                        snapshots.insert((visitor.current_block.unwrap(), index), captured);
                    }
                }
            }
            let spec =
                visitor
                    .deps
                    .require_dep::<MirSpecEnc>((def_id, def_id, MirSpecEncMode::Impure))?;
            if !spec.post_templates.is_empty() {
                let signature = RustSignature::new(def_id);
                let mut spec_args = Vec::new();
                for (index, ty) in signature.inputs.iter().enumerate() {
                    let normalized = ty.decompose_compare_normalize(signature.gparams, call_ctx);
                    let caster = visitor.deps.require_dep::<GArgsCastEnc<Pure>>(normalized)?;
                    spec_args.push(caster.cast_to_callee_ctx(args[(index + 1).into()]));
                }
                let normalized = signature
                    .output
                    .decompose_compare_normalize(signature.gparams, call_ctx);
                let caster = visitor.deps.require_dep::<GArgsCastEnc<Pure>>(normalized)?;
                let result = caster.cast_to_callee_ctx(args[mir::RETURN_PLACE]);
                let spec_args = PledgeExpr::pledge_args(result, spec_args);
                let bindings = &*vcx.alloc(std::cell::RefCell::new(Vec::new()));
                let capture = Some(PostCapture {
                    freeze_current: true,
                    label: Some(vir::OldLabel::Label(label_post)),
                    bindings: Some(bindings),
                    memo: Some(vcx.alloc(std::cell::RefCell::new(Default::default()))),
                });
                let generics = self.generics_subst(Some(call_ctx), vcx, visitor.deps);
                let facts = vcx.with_local_subst(generics, || {
                    spec.post_templates
                        .iter()
                        .map(|post| post.expr_at_label(spec_args, label_pre, capture))
                        .collect::<Vec<_>>()
                });
                // Select the stable consequence before emitting its observations.
                // Discarded postcondition branches may read a referent that exists
                // only under a guard (for example Option::Some after next()).
                let facts = facts
                    .into_iter()
                    .map(|fact| CallSnapshots::stable_consequence(vcx, fact))
                    .collect::<Vec<_>>();
                let mut substitute = captures.bind_used(bindings, &facts, visitor);
                let facts = facts
                    .into_iter()
                    .map(|fact| fact.fold_with(&mut substitute).unwrap_or(fact))
                    .collect();
                visitor
                    .call_post_facts
                    .insert(visitor.current_block.unwrap(), facts);
            }
        }
        Ok(wands)
    }

    /// Match an edge to the obligations actually emitted in the callee's
    /// contract. Resource-free shape edges need not have a corresponding wand.
    /// This does not classify opaque values as borrow-free: unknown mappings
    /// of an obligation's anchors are rejected, including pledge-only wands.
    pub(crate) fn call_wand_ids<E: TaskEncoder>(
        &self,
        argument_tys: &[ty::Ty<'vir>],
        result_ty: ty::Ty<'vir>,
        inputs: &[FunctionShapeNode],
        outputs: &[FunctionShapeNode],
        call_ctx: GArgs<'vir>,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, E>,
    ) -> Result<Vec<usize>, String> {
        if self.wands.is_empty() {
            return Ok(Vec::new());
        }
        let map = self
            .function_data
            .call_projection_map(argument_tys, result_ty, vcx.tcx())
            .map_err(|err| format!("cannot map function-call wand projections: {err:?}"))?;
        // Resource expressions use the definition-site group interface.
        // The mapping below must cover every concrete component of a group;
        // a partial edge cannot consume a whole-group restoration obligation.
        let mut selected = Vec::new();
        for (id, wand) in self.wands.iter().enumerate() {
            // Retain the definition-site interface, including opaque groups.
            let mut source_nodes = wand
                .rhs
                .iter()
                .copied()
                .map(Into::into)
                .filter(|node| {
                    self.predicates_for_function_shape_node(vcx, deps, *node, None)
                        .is_some()
                })
                .collect::<Vec<FunctionShapeNode<Generalized>>>();
            let pledge_only = source_nodes.is_empty();
            if pledge_only {
                source_nodes.extend(wand.rhs.iter().copied().map(FunctionShapeNode::from));
            }
            let mapped = |nodes: &[FunctionShapeNode<Generalized>]| -> Result<Vec<FunctionShapeNode>, String> {
                nodes.iter().try_fold(Vec::new(), |mut all, node| {
                    let targets = map.call_nodes(*node).ok_or_else(|| format!(
                        "cannot map function-call wand projection {node} through substitution"))?;
                    all.extend_from_slice(targets);
                    Ok(all)
                })
            };
            let sources = mapped(&source_nodes)?;
            if sources.is_empty() {
                // The contract still contains this wand. It need not be
                // applied when all opaque sources instantiate to checked empty
                // groups and it carries no caller obligation. Discarding its
                // additional postconditions only weakens the available facts;
                // an assert_on_expiry obligation must never be discarded.
                // Merely having no projection is insufficient for unknown or
                // collection types.
                let mut disposable = wand
                    .pledges
                    .iter()
                    .all(|pledge| pledge.expiry_obligation.is_none());
                for node in &source_nodes {
                    let regions: IndexVec<_, GeneralizedLifetime<'vir>> =
                        vcx.tcx().extract_regions(node.ty(self.fn_sig(vcx, None)));
                    let Some(GeneralizedLifetime::RegionsIn(opaque)) =
                        regions.get(node.region_idx())
                    else {
                        disposable = false;
                        break;
                    };
                    let actual = call_ctx.normalize(opaque.ty(vcx.tcx()));
                    let conversion = deps
                        .require_dep::<ResourceConversionEnc>(RustTyDecomposition::from_ty(
                            actual,
                            call_ctx.context(),
                        ))
                        .map_err(|_| {
                            "cannot check an empty instantiated resource group".to_string()
                        })?;
                    disposable &= conversion.has_trivial_resources();
                }
                if disposable {
                    continue;
                }
                return Err("function-call wand has no identifiable source projections".to_string());
            }
            if !sources.iter().any(|node| inputs.contains(node)) {
                continue;
            }
            // Stratification replaces earlier output dependencies with the
            // corresponding input resources as frames. The current edge must
            // cover the remaining targets; the wand still consumes and returns
            // every frame, so applying it checks that earlier restoration ran.
            // Keep mapping validation for those frames without requiring their
            // already discharged output nodes on this later PCG edge.
            let target_nodes = if wand.frames.is_empty() {
                wand.flow_lhs.as_ref().unwrap_or(&wand.lhs)
            } else {
                let frames = wand
                    .frames
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect::<Vec<_>>();
                let _ = mapped(&frames)?;
                &wand.lhs
            };
            let targets = target_nodes
                .iter()
                .copied()
                .filter(|node| {
                    pledge_only
                        || self
                            .predicates_for_function_shape_node(vcx, deps, *node, None)
                            .is_some()
                })
                .collect::<Vec<_>>();
            let targets = mapped(&targets)?;
            if wand.bundled_owner {
                // The original PCG still names the individual lifetime edges.
                // One such request consumes the complete checked obligation;
                // per-call/id/guard tracking prevents replay for another edge.
                // Reject any projection outside the combined owner interface.
                if !inputs.iter().all(|node| sources.contains(node))
                    || !outputs.iter().all(|node| targets.contains(node))
                {
                    return Err(
                        "whole-owner request exceeds its joint restoration interface".into(),
                    );
                }
                selected.push(id);
                continue;
            }
            if !sources.iter().all(|node| inputs.contains(node))
                || !targets.iter().all(|node| outputs.contains(node))
            {
                let list = |nodes: &[FunctionShapeNode]| {
                    nodes
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                return Err(format!(
                    "function-call edge covers only part of a restoration obligation \
                     (wand {id}: [{}] -> [{}]; edge: [{}] -> [{}])",
                    list(&sources),
                    list(&targets),
                    list(inputs),
                    list(outputs),
                ));
            }
            selected.push(id);
        }
        Ok(selected)
    }

    fn is_handoff_node(
        &self,
        node: WandRhsKey,
        wand: &WandData<'vir>,
        vcx: &'vir vir::VirCtxt<'vir>,
    ) -> bool {
        let shape: FunctionShapeNode<Generalized> = node.into();
        // Observe current resources only when this stage restores the owner
        // storage itself. Storage framed from an earlier stage may have been
        // changed by the caller in the meantime. That physical frame alone
        // does not supply the referents of the new value; return the entry
        // resources in that case instead of promising an unsupported handoff.
        node.region_idx().index() != 0
            && matches!(
                shape.ty(self.fn_sig(vcx, None)).kind(),
                ty::TyKind::Ref(_, _, ty::Mutability::Mut)
            )
            && wand.rhs.iter().any(|other| {
                other.mir_local() == node.mir_local() && other.region_idx().index() == 0
            })
    }

    fn input_owner<E: TaskEncoder>(
        &self,
        node: WandRhsKey,
        args: PledgeArgs<'vir>,
        call_ctx: WandCallContext<'vir>,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, E>,
    ) -> Option<OwnerIdentity<'vir>> {
        let storage = self.inputs.iter().find(|input| {
            input.mir_local() == node.mir_local() && input.region_idx().index() == 0
        })?;
        let resource = self.encode_predicates_for_function_shape_node(
            vcx,
            deps,
            *storage,
            call_ctx,
            None,
            None,
            |i| args[i],
        )?;
        struct Owner<'vir>(Option<OwnerIdentity<'vir>>);
        impl<'vir> Folder<'vir, (), !> for Owner<'vir> {
            fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                if let vir::ExprKindData::PredicateApp(app) = expr.kind {
                    if app.target == "p_Param" && self.0.is_none() {
                        self.0 = Some(OwnerIdentity {
                            address: app.args[0].downcast_ty(),
                            ty: app.args[1].downcast_ty(),
                        });
                    }
                }
                expr.super_fold_with(self)
            }
        }
        let mut owner = Owner(None);
        let _ = resource.fold_with(&mut owner);
        owner.0
    }

    fn owner_handoff<E: TaskEncoder>(
        &self,
        node: WandRhsKey,
        wand: &WandData<'vir>,
        args: PledgeArgs<'vir>,
        call_ctx: WandCallContext<'vir>,
        entry: vir::ExprBool<'vir>,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, E>,
    ) -> Option<OwnerHandoff<'vir>> {
        if !self.is_handoff_node(node, wand, vcx) {
            return None;
        }
        let owner = self.input_owner(node, args, call_ctx, vcx, deps)?;
        let captures = vcx.alloc(std::cell::RefCell::new(Vec::new()));
        let current = self.encode_predicates_for_function_shape_node(
            vcx,
            deps,
            node,
            call_ctx,
            None,
            Some(captures),
            |local| args[local],
        )?;
        // These observations are bound inside the outer wand's consequent,
        // after the owner storage is restored. Keep their names separate from
        // entry/post-call observations bound outside the wand.
        let mut substitute = HandoffSubstitute(FxHashMap::default());
        let mut bindings = Vec::new();
        for (index, &(decl, value)) in captures.borrow().iter().enumerate() {
            let value = value.fold_with(&mut substitute).unwrap_or(value);
            let name = vir::vir_format!(
                vcx,
                "_owner_current_{}_{}_{}",
                node.mir_local().index(),
                node.region_idx().index(),
                index
            );
            let saved = vcx.mk_local_decl(name, value.ty());
            bindings.push((saved, value));
            substitute
                .0
                .insert(decl.name, vcx.mk_local_ex(saved).as_dyn());
        }
        let current = current.fold_with(&mut substitute).unwrap_or(current);
        Some(OwnerHandoff {
            argument: node.mir_local(),
            owner,
            group: node.region_idx().index(),
            current,
            entry,
            bindings,
        })
    }

    fn mk_wand<E: TaskEncoder>(
        &self,
        wand_data: &WandData<'vir>,
        pledge_args: PledgeArgs<'vir>,
        pledge_old_label: Option<&'vir str>,
        output_heap_label: Option<&'vir str>,
        output_bindings: Option<&'vir SnapshotBindings<'vir>>,
        call_ctx: WandCallContext<'vir>,
        extra_pledges: &[PledgeExpr<'vir>],
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, E>,
    ) -> Option<vir::Wand<'vir>> {
        debug_assert!(!wand_data.lhs.is_empty() || !wand_data.frames.is_empty());
        let generics = self.generics_subst(call_ctx, vcx, deps);
        let spec_args = if let (Some(call_args), false) = (call_ctx, wand_data.pledges.is_empty()) {
            let signature = RustSignature::new(self.function_data.def_id());
            let arguments = signature
                .inputs
                .iter()
                .enumerate()
                .map(|(index, ty)| {
                    let normalized = ty.decompose_compare_normalize(signature.gparams, call_args);
                    let caster = deps.require_dep::<GArgsCastEnc<Pure>>(normalized).unwrap();
                    caster.cast_to_callee_ctx(pledge_args[(index + 1).into()])
                })
                .collect::<Vec<_>>();
            let normalized = signature
                .output
                .decompose_compare_normalize(signature.gparams, call_args);
            let caster = deps.require_dep::<GArgsCastEnc<Pure>>(normalized).unwrap();
            let result = caster.cast_to_callee_ctx(pledge_args[mir::RETURN_PLACE]);
            PledgeExpr::pledge_args(result, arguments)
        } else {
            pledge_args
        };
        let capture = Some(PostCapture {
            freeze_current: false,
            label: output_heap_label.map(vir::OldLabel::Label),
            bindings: output_bindings,
            memo: output_bindings.map(|_| &*vcx.alloc(std::cell::RefCell::new(Default::default()))),
        });
        let pledge_expr = |pledge: &PledgeExpr<'vir>| match pledge_old_label {
            Some(label) => {
                vcx.with_local_subst(generics, || pledge.expr_at_label(spec_args, label, capture))
            }
            None => pledge.expr(spec_args, capture),
        };
        let frames = wand_data
            .frames
            .iter()
            .filter_map(|g| {
                self.encode_predicates_for_function_shape_node(
                    vcx,
                    deps,
                    *g,
                    call_ctx,
                    Some(pledge_old_label.map_or(vir::OldLabel::None, vir::OldLabel::Label)),
                    output_bindings,
                    |i| pledge_args[i],
                )
            })
            .collect::<Vec<_>>();
        let frame_values = frames
            .iter()
            .map(|frame| physical_frame(vcx, *frame))
            .collect::<Vec<_>>();
        let mut transferred = Vec::new();
        let rhs = wand_data.rhs.iter().filter_map(|g| {
            let entry = self.encode_predicates_for_function_shape_node(
                vcx,
                deps,
                *g,
                call_ctx,
                Some(pledge_old_label.map_or(vir::OldLabel::None, vir::OldLabel::Label)),
                output_bindings,
                |i| pledge_args[i],
            )?;
            Some(
                match self.owner_handoff(*g, wand_data, pledge_args, call_ctx, entry, vcx, deps) {
                    Some(handoff) => handoff.consequence(vcx),
                    None => {
                        transferred.push(entry);
                        entry
                    }
                },
            )
        });
        // Establish physical frames before current-owner observations read
        // them. The conjunction retains exactly the same resources, but its
        // order must also satisfy Viper's self-framing check.
        let rhs = frames
            .iter()
            .copied()
            .chain(rhs)
            .chain(frame_values)
            .chain(
                wand_data
                    .pledges
                    .iter()
                    .map(|pledge| pledge_expr(&pledge.expiry_postcondition)),
            )
            // Conditional pledges are encoded in the caller's generic
            // context, so they use the caller's snapshots directly rather
            // than the cast and substitution used for the base pledge.
            .chain(extra_pledges.iter().map(|pledge| match pledge_old_label {
                Some(label) => pledge.expr_at_label(pledge_args, label, capture),
                None => pledge.expr(pledge_args, capture),
            }))
            .collect::<Vec<_>>();
        if rhs.is_empty() {
            // We skip emitting the wand when there is nothing on the RHS, i.e.,
            // nothing would be unblocked by applying this wand, nor are there
            // any pledge postconditions.
            return None;
        }
        let rhs = vcx.mk_conj(&rhs);
        let lhs = wand_data.lhs.iter().filter_map(|g| {
            self.encode_predicates_for_function_shape_node(
                vcx,
                deps,
                *g,
                call_ctx,
                output_heap_label.map(vir::OldLabel::Label),
                output_bindings,
                |i| pledge_args[i],
            )
        });
        let lhs = lhs
            .chain(frames)
            .chain(
                wand_data
                    .pledges
                    .iter()
                    .filter_map(|pledge| pledge.expiry_obligation.as_ref().map(pledge_expr)),
            )
            .collect::<Vec<_>>();
        let lhs = vcx.mk_conj(&lhs);
        let transfer_values = transfer_frames(vcx, lhs, vcx.mk_conj(&transferred));
        let rhs = if matches!(
            transfer_values.kind,
            vir::ExprKindData::Const(vir::ConstData::Bool(true))
        ) {
            rhs
        } else {
            vcx.mk_conj(&[rhs, transfer_values])
        };
        Some(vcx.mk_wand(lhs, rhs))
    }

    /// The pledges are encoded once, at the callee's identity substitution,
    /// so they refer to the callee's generic parameters. At a call site, these
    /// are replaced by the call's generic arguments, as Viper does for the
    /// callee's postcondition (from which the caller holds the wand).
    fn generics_subst<E: TaskEncoder>(
        &self,
        call_ctx: WandCallContext<'vir>,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, E>,
    ) -> &'vir FxHashMap<&'vir str, vir::ExprDyn<'vir>> {
        let Some(call_args) = call_ctx else {
            return vcx.alloc(FxHashMap::default());
        };
        let params = deps
            .require_dep::<GenericParamsEnc>(self.g_params(vcx, None))
            .unwrap();
        let args = deps.require_dep::<GArgsTyEnc>(call_args).unwrap();
        let tys = params
            .ty_decls()
            .iter()
            .zip(args.get_ty::<(), !>())
            .map(|(decl, arg)| (decl.name, arg.as_dyn()));
        let consts = params
            .const_decls()
            .iter()
            .zip(args.get_const::<(), !>())
            .map(|(decl, arg)| (decl.name, arg.as_dyn()));
        vcx.alloc(tys.chain(consts).collect())
    }
}

/// When an expiry transfers a resource already supplied by its antecedent,
/// preserve its observed state. The guard compares the complete resource key;
/// each observation is framed by permissions on its own side of the wand.
/// This also handles permutations justified by functional postconditions.
/// Packages in untrusted callees must prove these equalities.
/// Returning an owner's entry resources must preserve the values of complete
/// resource identities supplied by the continuation's antecedent. Apply the
/// same checked frame rule to handoffs and composed loop summaries as to the
/// outer function wands; otherwise applying a handoff discards those facts.
/// If both sides name the same complete flat resource footprint, the
/// antecedent itself proves restoration. Replaying a chain of opaque wands
/// would unnecessarily forget its current observations. This is a proof
/// branch only: the original consequent, including every value fact, remains
/// checked. More complex/quantified layouts keep their original proof script.
/// Select the exact opaque leaves for known struct-field paths. The complete
/// enclosing footprint must be covered; conditional/quantified layouts and
/// ambiguous projections keep the original whole-owner history.
fn projected_resource_parts<'vir>(
    current: vir::ExprBool<'vir>,
    entry: vir::ExprBool<'vir>,
    paths: &[&'vir [&'vir str]],
) -> Option<Vec<(&'vir [&'vir str], vir::ExprBool<'vir>, vir::ExprBool<'vir>)>> {
    fn leaves<'vir>(expr: vir::ExprBool<'vir>, out: &mut Vec<vir::ExprBool<'vir>>) -> bool {
        match expr.kind {
            vir::ExprKindData::PredicateApp(app) if app.target == "indirect_resources" => {
                out.push(expr);
                true
            }
            vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => {
                leaves(op.lhs.downcast_ty(), out) && leaves(op.rhs.downcast_ty(), out)
            }
            vir::ExprKindData::Const(vir::ConstData::Bool(true)) => true,
            _ => false,
        }
    }
    fn matches_path(expr: vir::ExprBool<'_>, path: &[&str]) -> bool {
        let vir::ExprKindData::PredicateApp(app) = expr.kind else {
            return false;
        };
        let mut value = app.args[0];
        for expected in path.iter().rev() {
            loop {
                let vir::ExprKindData::AdtDestructor(inner, field) = value.kind else {
                    return false;
                };
                value = inner;
                if field.name == *expected {
                    break;
                }
                if !field.name.starts_with("make_concrete_s_") {
                    return false;
                }
            }
        }
        !path.is_empty()
    }
    let mut before = Vec::new();
    let mut after = Vec::new();
    if paths.is_empty()
        || !leaves(current, &mut before)
        || !leaves(entry, &mut after)
        || before.len() != paths.len()
        || after.len() != paths.len()
    {
        return None;
    }
    let mut result = Vec::new();
    for &path in paths {
        let left = before
            .iter()
            .enumerate()
            .filter(|(_, value)| matches_path(**value, path))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let right = after
            .iter()
            .enumerate()
            .filter(|(_, value)| matches_path(**value, path))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let ([left], [right]) = (left.as_slice(), right.as_slice()) else {
            return None;
        };
        result.push((path, before.remove(*left), after.remove(*right)));
    }
    (before.is_empty() && after.is_empty()).then_some(result)
}

fn same_resource_footprint<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    current: vir::ExprBool<'vir>,
    entry: vir::ExprBool<'vir>,
) -> Option<vir::ExprBool<'vir>> {
    fn leaves<'vir>(expr: vir::ExprBool<'vir>, out: &mut Vec<vir::ExprBool<'vir>>) -> bool {
        match expr.kind {
            vir::ExprKindData::PredicateApp(app)
                if matches!(app.target, "p_Param" | "indirect_resources") =>
            {
                out.push(expr);
                true
            }
            vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => {
                leaves(op.lhs.downcast_ty(), out) && leaves(op.rhs.downcast_ty(), out)
            }
            _ => {
                struct Pure(bool);
                impl<'vir> Folder<'vir, (), !> for Pure {
                    fn fold_expr(
                        &mut self,
                        expr: vir::ExprDyn<'vir>,
                    ) -> Option<vir::ExprDyn<'vir>> {
                        if matches!(
                            expr.kind,
                            vir::ExprKindData::PredicateApp(_)
                                | vir::ExprKindData::PredicatePerm(_)
                                | vir::ExprKindData::AccField(_)
                                | vir::ExprKindData::Wand(_)
                                | vir::ExprKindData::InhaleExhale(_)
                        ) {
                            self.0 = false;
                        }
                        expr.super_fold_with(self)
                    }
                }
                let mut pure = Pure(true);
                let _ = expr.fold_with(&mut pure);
                pure.0
            }
        }
    }
    let mut before = Vec::new();
    let mut after = Vec::new();
    if !leaves(current, &mut before)
        || !leaves(entry, &mut after)
        || before.is_empty()
        || before.len() != after.len()
    {
        return None;
    }
    let mut equalities = Vec::new();
    for (before, after) in before.into_iter().zip(after) {
        let vir::ExprKindData::PredicateApp(before) = before.kind else {
            unreachable!()
        };
        let vir::ExprKindData::PredicateApp(after) = after.kind else {
            unreachable!()
        };
        if before.target != after.target || before.args.len() != after.args.len() {
            return None;
        }
        equalities.extend(
            before
                .args
                .iter()
                .zip(after.args)
                .map(|(&a, &b)| vcx.mk_eq_expr(a, b)),
        );
    }
    Some(vcx.mk_conj(&equalities))
}

fn direct_resource_restoration<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    current: vir::ExprBool<'vir>,
    entry: vir::ExprBool<'vir>,
    script: Vec<vir::Stmt<'vir>>,
) -> Vec<vir::Stmt<'vir>> {
    match same_resource_footprint(vcx, current, entry) {
        Some(same) => vec![vcx.mk_if_stmt(same, &[], vcx.alloc_slice(&script))],
        None => script,
    }
}

/// Keep an equal-key current resource framed until the other owner's entry
/// resource has been restored. Applying that other history first gives two
/// permissions to the same resource; its existing observation then remains
/// framed when the next history consumes one permission. Complete permutations
/// already satisfy the resource obligation without applying either history.
/// Every branch still proves the original enclosing wand, including all frames.
fn ordered_owner_restoration<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    owners: Vec<(
        vir::ExprBool<'vir>,
        vir::ExprBool<'vir>,
        Vec<vir::Stmt<'vir>>,
    )>,
) -> Vec<vir::Stmt<'vir>> {
    if owners.len() == 2 {
        let mut equalities = Vec::new();
        for before in &owners {
            for after in &owners {
                if let Some(same) = same_resource_footprint(vcx, before.0, after.1) {
                    equalities.push(same);
                }
            }
        }
        if equalities.len() == 4 {
            fn branches<'vir>(
                vcx: &'vir vir::VirCtxt<'vir>,
                equalities: &[vir::ExprBool<'vir>],
                owners: &[(
                    vir::ExprBool<'vir>,
                    vir::ExprBool<'vir>,
                    Vec<vir::Stmt<'vir>>,
                )],
                choices: &mut Vec<bool>,
            ) -> Vec<vir::Stmt<'vir>> {
                if choices.len() == 4 {
                    if (choices[0] && choices[3]) || (choices[1] && choices[2]) {
                        return Vec::new();
                    }
                    let order = if choices[1] && !choices[2] {
                        [1, 0]
                    } else {
                        [0, 1]
                    };
                    let mut script = Vec::new();
                    for owner in order {
                        if !choices[owner * 3] {
                            script.extend_from_slice(&owners[owner].2);
                        }
                    }
                    return script;
                }
                let condition = equalities[choices.len()];
                choices.push(true);
                let yes = branches(vcx, equalities, owners, choices);
                *choices.last_mut().unwrap() = false;
                let no = branches(vcx, equalities, owners, choices);
                choices.pop();
                vec![vcx.mk_if_stmt(condition, vcx.alloc_slice(&yes), vcx.alloc_slice(&no))]
            }
            return branches(vcx, &equalities, &owners, &mut Vec::new());
        }
    }
    owners
        .into_iter()
        .flat_map(|(current, entry, script)| {
            direct_resource_restoration(vcx, current, entry, script)
        })
        .collect()
}

fn resource_restoration_wand<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    current: vir::ExprBool<'vir>,
    entry: vir::ExprBool<'vir>,
) -> vir::Wand<'vir> {
    let values = transfer_frames(vcx, current, entry);
    let rhs = if matches!(
        values.kind,
        vir::ExprKindData::Const(vir::ConstData::Bool(true))
    ) {
        entry
    } else {
        vcx.mk_conj(&[entry, values])
    };
    vcx.mk_wand(current, rhs)
}

fn transfer_frames<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    lhs: vir::ExprBool<'vir>,
    rhs: vir::ExprBool<'vir>,
) -> vir::ExprBool<'vir> {
    fn stable_key<'vir>(expr: vir::ExprDyn<'vir>) -> bool {
        struct Check(bool);
        impl<'vir> Folder<'vir, (), !> for Check {
            fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                match expr.kind {
                    vir::ExprKindData::Old(old) if old.label != vir::OldLabel::Lhs => {
                        // A named/entry heap observation is already fixed at
                        // wand creation, independently of the eventual LHS.
                        return Some(expr);
                    }
                    vir::ExprKindData::Old(_)
                    | vir::ExprKindData::Field(..)
                    | vir::ExprKindData::PredicateApp(_)
                    | vir::ExprKindData::PredicatePerm(_)
                    | vir::ExprKindData::Unfolding(_)
                    | vir::ExprKindData::AccField(_)
                    | vir::ExprKindData::Wand(_)
                    | vir::ExprKindData::InhaleExhale(_) => self.0 = false,
                    // VIR represents generated snapshot/type constructors and
                    // pure snapshot operations as FuncApp as well. Their s_
                    // namespace (and generic snapshot injections) has no heap
                    // precondition. Generated associated-type projections are
                    // also domain functions over generic arguments (TraitEnc),
                    // including Pointee::Metadata. Other functions stay excluded.
                    vir::ExprKindData::FuncApp(app)
                        if !app.target.starts_with("s_")
                            && !app.target.starts_with("make_generic_s_")
                            && !(app.result_ty == vir::TYPE_TYVAL.as_dyn()
                                && app.target.contains("_assoc_type_")) =>
                    {
                        self.0 = false
                    }
                    _ => {}
                }
                expr.super_fold_with(self)
            }
        }
        let mut check = Check(true);
        let _ = expr.fold_with(&mut check);
        check.0
    }
    struct Leaf<'vir> {
        permission: vir::ExprBool<'vir>,
        guard: vir::ExprBool<'vir>,
        variables: &'vir [vir::LocalDeclDyn<'vir>],
        triggers: &'vir [vir::Trigger<'vir>],
    }
    fn leaves<'vir>(
        vcx: &'vir vir::VirCtxt<'vir>,
        expr: vir::ExprBool<'vir>,
        guard: vir::ExprBool<'vir>,
        variables: &'vir [vir::LocalDeclDyn<'vir>],
        triggers: &'vir [vir::Trigger<'vir>],
        out: &mut Vec<Leaf<'vir>>,
    ) {
        match expr.kind {
            vir::ExprKindData::PredicateApp(app)
                if matches!(
                    app.target,
                    "p_Param"
                        | "indirect_resources"
                        | "indexed_p_Param"
                        | "indexed_indirect_resources"
                ) =>
            {
                out.push(Leaf {
                    permission: expr,
                    guard,
                    variables,
                    triggers,
                })
            }
            vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => {
                leaves(vcx, op.lhs.downcast_ty(), guard, variables, triggers, out);
                leaves(vcx, op.rhs.downcast_ty(), guard, variables, triggers, out);
            }
            vir::ExprKindData::BinOp(op)
                if op.kind == vir::BinOpKind::Implies && stable_key(op.lhs) =>
            {
                let guard = vcx.mk_conj(&[guard, op.lhs.downcast_ty()]);
                leaves(vcx, op.rhs.downcast_ty(), guard, variables, triggers, out);
            }
            vir::ExprKindData::Forall(quantified) if variables.is_empty() => {
                leaves(
                    vcx,
                    quantified.body,
                    guard,
                    quantified.qvars,
                    quantified.triggers,
                    out,
                );
            }
            // Keep a single quantified scope. Nested binders, let expressions
            // and conditional branches need separate correspondence rules.
            _ => {}
        }
    }
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    leaves(vcx, lhs, vcx.mk_bool::<true>(), &[], &[], &mut inputs);
    leaves(vcx, rhs, vcx.mk_bool::<true>(), &[], &[], &mut outputs);
    let mut facts = Vec::new();
    for output in outputs {
        let vir::ExprKindData::PredicateApp(after) = output.permission.kind else {
            unreachable!()
        };
        for input in &inputs {
            if !output.variables.is_empty()
                && !input.variables.is_empty()
                && (output.variables.len() != input.variables.len()
                    || output
                        .variables
                        .iter()
                        .zip(input.variables)
                        .any(|(a, b)| a.ty != b.ty))
            {
                continue;
            }
            // Compare the two resource families at the same bound indices,
            // alpha-renaming the antecedent into the consequent's scope. Both
            // range guards remain, and complete resource keys must still match.
            let mut rename = SnapshotSubstitute(
                input
                    .variables
                    .iter()
                    .zip(output.variables)
                    .map(|(before, after)| (before.name, vcx.mk_local_ex(after)))
                    .collect(),
            );
            let before_permission = input
                .permission
                .fold_with(&mut rename)
                .unwrap_or(input.permission);
            let before_guard = input.guard.fold_with(&mut rename).unwrap_or(input.guard);
            let vir::ExprKindData::PredicateApp(before) = before_permission.kind else {
                unreachable!()
            };
            let (after_kind, after_args) = if let Some(kind) = after.target.strip_prefix("indexed_")
            {
                (kind, &after.args[2..])
            } else {
                (after.target, after.args)
            };
            let (before_kind, before_args) =
                if let Some(kind) = before.target.strip_prefix("indexed_") {
                    (kind, &before.args[2..])
                } else {
                    (before.target, before.args)
                };
            if after_kind != before_kind || after_args.len() != before_args.len() {
                continue;
            }
            // An indexed wrapper partitions ownership of its underlying
            // resource. A scalar/collection transfer can retag that wrapper;
            // compare its full underlying identity while retaining the range
            // guards. For two indexed families retain the index/path keys too.
            let (after_args, before_args) = if after.target == before.target {
                (after.args, before.args)
            } else {
                (after_args, before_args)
            };
            if !after_args
                .iter()
                .chain(before_args)
                .all(|arg| stable_key(*arg))
            {
                continue;
            }
            let keys = after_args
                .iter()
                .zip(before_args)
                .map(|(&a, &b)| vcx.mk_eq_expr(a, b))
                .collect::<Vec<_>>();
            let guard = vcx.mk_conj(&[output.guard, before_guard, vcx.mk_conj(&keys)]);
            let vir::ExprKindData::BinOp(after_frame) = physical_frame(vcx, output.permission).kind
            else {
                unreachable!()
            };
            let vir::ExprKindData::BinOp(before_frame) =
                physical_frame(vcx, before_permission).kind
            else {
                unreachable!()
            };
            let preserved = vcx.mk_eq_expr(after_frame.lhs, before_frame.rhs);
            let fact = vcx
                .mk_bin_op_expr(vir::BinOpKind::Implies, guard, preserved)
                .downcast_ty();
            let (variables, triggers) = if output.variables.is_empty() {
                (input.variables, input.triggers)
            } else {
                (output.variables, output.triggers)
            };
            facts.push(if variables.is_empty() {
                fact
            } else {
                vcx.mk_forall_expr(variables, triggers, fact)
            });
        }
    }
    vcx.mk_conj(&facts)
}

/// A frame preserves its heap state as well as permission. Mutable-reference
/// leaves use `p_Param`; opaque groups expose a recursive resource state.
fn physical_frame<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    resources: vir::ExprBool<'vir>,
) -> vir::ExprBool<'vir> {
    match resources.kind {
        vir::ExprKindData::PredicateApp(app)
            if app.target == "indexed_p_Param" || app.target == "indexed_indirect_resources" =>
        {
            // The index and path distinguish logical ownership even when
            // reference addresses coincide. Read through the same wrapper on
            // both heaps; observing a frame must not open or consume it.
            let state = if app.target == "indexed_p_Param" {
                assert_eq!(app.args.len(), 4);
                vir::FunctionIdn::<(vir::Ref, vir::TyVal), vir::PSnap>::new(
                    vir::ViperIdent::new("p_Param_snap"),
                    (vir::TYPE_REF, vir::TYPE_TYVAL),
                    vir::TYPE_PSNAP,
                )
                .call()(app.args[2].downcast_ty(), app.args[3].downcast_ty())
                .as_dyn()
            } else {
                assert_eq!(app.args.len(), 5);
                crate::encoders::ty::resources::state_function("indirect_resource_state").call()(
                    app.args[2].downcast_ty(),
                    app.args[3].downcast_ty(),
                    app.args[4].downcast_ty(),
                )
                .as_dyn()
            };
            let state = vcx.mk_unfolding_expr(app, state);
            vcx.mk_eq_expr(state, vcx.mk_old_lhs_expr(state))
        }
        vir::ExprKindData::PredicateApp(app) if app.target == "indirect_resources" => {
            let state = crate::encoders::ty::resources::state_function("indirect_resource_state")
                .call()(
                app.args[0].downcast_ty(),
                app.args[1].downcast_ty(),
                app.args[2].downcast_ty(),
            );
            vcx.mk_eq_expr(state, vcx.mk_old_lhs_expr(state))
        }
        vir::ExprKindData::PredicateApp(app) => {
            assert_eq!(
                app.target, "p_Param",
                "unsupported dependent resource frame"
            );
            assert_eq!(app.args.len(), 2);
            let snap = vir::FunctionIdn::<(vir::Ref, vir::TyVal), vir::PSnap>::new(
                vir::ViperIdent::new("p_Param_snap"),
                (vir::TYPE_REF, vir::TYPE_TYVAL),
                vir::TYPE_PSNAP,
            )
            .call()(app.args[0].downcast_ty(), app.args[1].downcast_ty());
            vcx.mk_eq_expr(snap, vcx.mk_old_lhs_expr(snap))
        }
        vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => vcx.mk_conj(&[
            physical_frame(vcx, op.lhs.downcast_ty()),
            physical_frame(vcx, op.rhs.downcast_ty()),
        ]),
        vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::Implies => {
            let frame = physical_frame(vcx, op.rhs.downcast_ty());
            let guard = op.lhs.downcast_ty::<vir::Bool>();
            vir::expr! { (guard) ==> (frame) }
        }
        vir::ExprKindData::Forall(quantified) => vcx.mk_forall_expr(
            quantified.qvars,
            quantified.triggers,
            physical_frame(vcx, quantified.body),
        ),
        vir::ExprKindData::Ternary(branch) => vcx.mk_ternary_expr(
            branch.cond,
            physical_frame(vcx, branch.then.downcast_ty()),
            physical_frame(vcx, branch.else_.downcast_ty()),
        ),
        vir::ExprKindData::Let(binding) => vcx.mk_let_expr(
            vcx.mk_local_decl(binding.name, binding.val.ty()),
            binding.val,
            physical_frame(vcx, binding.expr.downcast_ty()),
        ),
        _ => vcx.mk_bool::<true>(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WandEncTask<'tcx> {
    pub data: FunctionData<'tcx>,
}

impl<'tcx> WandEncTask<'tcx> {
    pub fn def_id(&self) -> DefId {
        self.data.def_id()
    }

    pub fn function_shape(
        &self,
        vcx: &vir::VirCtxt<'tcx>,
    ) -> Result<FunctionShape<Generalized>, MakeFunctionShapeError> {
        self.data.shape(vcx.tcx())
    }
}

pub type WandRhsKey = FunctionShapeInput<Generalized>;
pub type WandLhsKey = FunctionShapeNode<Generalized>;

#[derive(Clone, Debug)]
pub struct WandData<'vir> {
    /// Lifetime projections on the right-hand side of the wand. Guaranteed to be
    /// non-empty.
    rhs: Vec<WandRhsKey>,
    /// Lifetime projections on the left-hand side. Non-empty unless earlier
    /// stages supply the complete checked antecedent through `frames`.
    lhs: Vec<WandLhsKey>,
    pledges: EncodedPledges<'vir>,
    /// Permissions restored by an earlier wand, consumed and returned unchanged.
    frames: Vec<WandRhsKey>,
    /// Original shape targets retained for checking PCG call-edge coverage.
    flow_lhs: Option<Vec<WandLhsKey>>,
    /// Joint input-owner restoration: every antecedent group is checked.
    bundled_owner: bool,
}

impl<'vir> WandData<'vir> {
    pub fn new(lhs: Vec<WandLhsKey>, rhs: Vec<WandRhsKey>, pledges: EncodedPledges<'vir>) -> Self {
        debug_assert!(!lhs.is_empty());
        debug_assert!(!rhs.is_empty());
        Self {
            rhs,
            lhs,
            pledges,
            frames: Vec::new(),
            flow_lhs: None,
            bundled_owner: false,
        }
    }
}

impl TaskEncoder for WandEnc {
    task_encoder::encoder_cache!(WandEnc);

    type TaskDescription<'vir> = WandEncTask<'vir>;

    type TaskKey<'vir> = WandEncTask<'vir>;

    type OutputFullDependency<'vir> = WandEncOutput<'vir>;

    type EncodingError = WandEncError;

    const ENCODER_NAME: &'static str = "wand encoder";

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        task.clone()
    }

    fn describe_error(error: Self::EncodingError) -> String {
        match error {
            WandEncError::Unsupported(message) => message,
        }
    }

    fn do_encode_full<'vir>(
        task_key: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(task_key.clone(), ())?;
        vir::with_vcx(|vcx| {
            let def_id = task_key.def_id();

            let shape = task_key.function_shape(vcx).map_err(|e| {
                EncodeFullError::EncodingError(
                    WandEncError::Unsupported(format!("function shape: {e:?}")),
                    None,
                )
            })?;

            let coupled_edges = shape.coupled_edges();
            let edges: FxHashSet<_> = shape.edges().map(|e| (e.input(), e.output())).collect();

            let (inputs, outputs) = shape.take_inputs_and_outputs();
            let spec = deps.require_dep::<MirSpecEnc>((def_id, def_id, MirSpecEncMode::Impure))?;
            if coupled_edges.is_empty() {
                if !spec.pledges.is_empty() {
                    return Err(EncodeFullError::EncodingError(
                        WandEncError::Unsupported(
                            "expiry pledge has no borrowed input to restore".into(),
                        ),
                        None,
                    ));
                }
                return Ok((
                    (),
                    WandEncOutput {
                        function_data: task_key.data,
                        inputs,
                        outputs,
                        wands: vec![],
                    },
                ));
            }
            let pledges = spec.pledges;
            let wands: Vec<WandData<'vir>> = coupled_edges
                .into_iter()
                .map(|hyper_edge| {
                    let (sources, targets) = hyper_edge.into_tuple();
                    // Equal signature nodes need not denote equal resources:
                    // a mutable argument can contain different references on
                    // return. Keep its post-call resources and the obligation
                    // to restore its pre-call resources distinct. Selection
                    // below removes outputs belonging to consumed arguments.
                    WandData::new(targets, sources, pledges.clone())
                })
                .collect();
            let mut output: WandEncOutput<'vir> = WandEncOutput {
                function_data: task_key.data,
                inputs,
                outputs,
                wands: Vec::new(),
            };
            output.wands = output
                .select_wands(wands, &pledges, &edges, vcx, deps)
                .map_err(|err| EncodeFullError::EncodingError(err, None))?;
            Ok(((), output))
        })
    }
}

impl<'vir> WandEncOutput<'vir> {
    /// Selects the wands to emit among those of the coupled edges, rejecting
    /// the shapes that cannot be encoded precisely.
    fn select_wands<E: TaskEncoder>(
        &self,
        wands: Vec<WandData<'vir>>,
        pledges: &[EncodedPledge<'vir>],
        edges: &FxHashSet<(WandRhsKey, WandLhsKey)>,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, E>,
    ) -> Result<Vec<WandData<'vir>>, WandEncError> {
        let mut has_resources = |g: FunctionShapeNode<Generalized>| {
            self.predicates_for_function_shape_node(vcx, deps, g, None)
                .is_some()
        };
        let sig = self.fn_sig(vcx, None);
        // Generic results also carry signature-only lifetime anchors. Those
        // projections have no resource predicate and do not keep an input's
        // storage borrowed. Only a resource-bearing result can hide the
        // nested post-call resources of a mutable argument. In particular,
        // mem::replace must return both its result's resources and the new
        // resources in the destination.
        let borrowed_storage = edges
            .iter()
            .filter(|(source, target)| {
                source.region_idx().index() == 0
                    && target.mir_local() == mir::RETURN_PLACE
                    && has_resources(*target)
            })
            .map(|(source, _)| source.mir_local())
            .collect::<FxHashSet<_>>();
        let visible = |node: WandLhsKey| {
            if node.mir_local() == mir::RETURN_PLACE {
                return true;
            }
            if !matches!(
                node.ty(sig).kind(),
                ty::TyKind::Ref(_, _, ty::Mutability::Mut)
            ) {
                return false;
            }
            // An input's nested resources cannot also be returned directly
            // while its outer storage remains borrowed by the result.
            !borrowed_storage.contains(&node.mir_local())
        };
        let wands = wands
            .into_iter()
            .filter_map(|mut wand| {
                wand.lhs.retain(|node| visible(*node));
                // A consumed argument with no visible output does not have
                // an expiry stage. Return its resources directly instead of
                // attaching an unrelated pledge to a vacuous self-edge.
                // This also keeps an unused generic marker from making an
                // otherwise unambiguous pledge appear to need a selector.
                (!wand.lhs.is_empty()).then_some(wand)
            })
            .collect::<Vec<_>>();
        // A resource-free LHS without an expiry obligation encodes as `true`.
        // Return its resources directly: `true --* P` can be applied immediately,
        // but keeping P inside the wand prevents functional postconditions from
        // reading it. This occurs for e.g. `mem::replace<T>(&mut T, T) -> T`,
        // whose generalized result projections have no indirect predicates.
        // Removing the wand also removes its RHS from `blocked_inputs`, so
        // `indirect_posts` returns the corresponding permissions instead.
        // Keep pledges: their conditions belong to the borrow's expiry, even
        // when the reference itself carries no encoded permissions.
        let wands = wands
            .into_iter()
            .filter(|wand_data| {
                !wand_data.pledges.is_empty() || wand_data.lhs.iter().any(|g| has_resources(*g))
            })
            .collect::<Vec<_>>();
        // A wand is only needed if it gives back a resource, or to carry the
        // pledges if none does (e.g. for shared references).
        let (wands, resourceless): (Vec<_>, Vec<_>) = wands
            .into_iter()
            .partition(|wand_data| wand_data.rhs.iter().any(|g| has_resources((*g).into())));
        // A wand gives back its sources only once all of its targets have
        // expired. That matches the signature only if each source with
        // resources flows into each target with resources.
        let mut stratified = Vec::new();
        for wand_data in wands {
            let sources = wand_data
                .rhs
                .iter()
                .copied()
                .filter(|g| has_resources((*g).into()))
                .collect::<Vec<_>>();
            let targets = wand_data
                .lhs
                .iter()
                .copied()
                .filter(|g| has_resources(*g))
                .collect::<Vec<_>>();
            if sources.iter().all(|source| {
                targets
                    .iter()
                    .all(|target| edges.contains(&(*source, *target)))
            }) {
                stratified.push(wand_data);
                continue;
            }
            // Nested target sets admit an ordered restoration: replace the
            // targets restored earlier with their source permissions, and
            // frame those permissions through the later wand.
            let mut groups: Vec<(Vec<WandLhsKey>, Vec<WandRhsKey>)> = Vec::new();
            for source in sources {
                let mut targets = targets
                    .iter()
                    .copied()
                    .filter(|target| edges.contains(&(source, *target)))
                    .collect::<Vec<_>>();
                targets.sort();
                if let Some((_, sources)) =
                    groups.iter_mut().find(|(existing, _)| *existing == targets)
                {
                    sources.push(source);
                } else {
                    groups.push((targets, vec![source]));
                }
            }
            // An opaque restored input is not a replacement for the resources
            // of its outputs: after instantiation it may even be empty (e.g.
            // the usize index of IndexMut). Factoring a later restoration
            // through that input loses the output's state. Keep overlapping
            // nested-resource groups together. Physical owner storage can
            // still be restored earlier, and disjoint groups remain separate.
            let has_storage = |sources: &[WandRhsKey]| {
                sources.iter().any(|source| {
                    let node: FunctionShapeNode<Generalized> = (*source).into();
                    // A nested reference (for example the slice inside
                    // Option<&mut [T]>) has physical storage too. Classify
                    // the resource group itself, not just the outer argument.
                    let regions: IndexVec<_, GeneralizedLifetime<'vir>> =
                        vcx.tcx().extract_regions(node.ty(sig));
                    !matches!(
                        regions.get(source.region_idx()),
                        Some(GeneralizedLifetime::RegionsIn(_))
                    )
                })
            };
            loop {
                let pair = (0..groups.len()).find_map(|i| {
                    (i + 1..groups.len()).find_map(|j| {
                        (!has_storage(&groups[i].1)
                            && !has_storage(&groups[j].1)
                            && groups[i]
                                .0
                                .iter()
                                .any(|target| groups[j].0.contains(target)))
                        .then_some((i, j))
                    })
                });
                let Some((i, j)) = pair else { break };
                let (targets, sources) = groups.remove(j);
                groups[i].0.extend(targets);
                groups[i].1.extend(sources);
                groups[i].0.sort();
                groups[i].0.dedup();
                groups[i].1.sort();
                groups[i].1.dedup();
            }
            groups.sort_by_key(|(targets, _)| targets.len());
            let mut previous: Vec<(Vec<WandLhsKey>, Vec<WandRhsKey>)> = Vec::new();
            for (original, sources) in groups {
                // This source has no resource-bearing output dependency. With
                // no expiry pledge to anchor, return it directly through
                // indirect_posts instead of inventing an empty restoration
                // stage (e.g. the destination storage of mem::replace<T>).
                if original.is_empty() && pledges.is_empty() {
                    continue;
                }
                let mut targets = original.clone();
                let mut frames = Vec::new();
                for (earlier_targets, earlier_sources) in &previous {
                    let overlap = earlier_targets
                        .iter()
                        .any(|target| original.contains(target));
                    if overlap
                        && !earlier_targets
                            .iter()
                            .all(|target| original.contains(target))
                    {
                        return Err(WandEncError::Unsupported(
                            "overlapping resource restoration dependencies are not nested".into(),
                        ));
                    }
                    if overlap {
                        targets.retain(|target| !earlier_targets.contains(target));
                        frames.extend(earlier_sources.iter().copied());
                    }
                }
                if targets.is_empty() && frames.is_empty() {
                    return Err(WandEncError::Unsupported(
                        "empty dependent resource restoration".into(),
                    ));
                }
                previous.push((original.clone(), sources.clone()));
                // Earlier stages may supply the complete checked antecedent.
                let mut wand = WandData::new(original.clone(), sources, Vec::new());
                wand.lhs = targets;
                wand.frames = frames;
                wand.flow_lhs = Some(original);
                stratified.push(wand);
            }
        }
        // A void function has no escaped output loan. Retain one joint restoration
        // obligation for each multi-group owner: an opaque generic callee can
        // return an indivisible continuation over the instantiated product.
        // That continuation cannot be factored into separate group wands.
        // This strengthens the required expiry antecedent; it grants no
        // resources and introduces no facts. Every combined package and every
        // application must prove all dependent groups simultaneously.
        if sig.output().is_unit() && pledges.is_empty() {
            for (index, input) in sig.inputs().iter().enumerate() {
                let ty::TyKind::Ref(_, _, ty::Mutability::Mut) = input.kind() else {
                    continue;
                };
                let local = mir::Local::from_usize(index + 1);
                let selected =
                    stratified
                        .iter()
                        .enumerate()
                        .filter_map(|(id, wand)| {
                            let only_owner =
                                wand.rhs.iter().all(|node| {
                                    node.mir_local() == local && node.region_idx().index() > 0
                                }) && wand.flow_lhs.as_ref().unwrap_or(&wand.lhs).iter().all(
                                    |node| {
                                        node.mir_local() == local && node.region_idx().index() > 0
                                    },
                                ) && wand.frames.iter().all(|node| {
                                    node.mir_local() == local && node.region_idx().index() > 0
                                });
                            only_owner.then_some(id)
                        })
                        .collect::<Vec<_>>();
                if selected.len() < 2 {
                    continue;
                }
                let mut lhs = Vec::new();
                let mut rhs = Vec::new();
                for &id in &selected {
                    let wand = &stratified[id];
                    lhs.extend_from_slice(wand.flow_lhs.as_ref().unwrap_or(&wand.lhs));
                    rhs.extend_from_slice(&wand.rhs);
                }
                lhs.sort();
                lhs.dedup();
                rhs.sort();
                rhs.dedup();
                // No parent frame may be dropped unless it is part of this
                // joint input/output resource set.
                if selected
                    .iter()
                    .any(|&id| stratified[id].frames.iter().any(|node| !rhs.contains(node)))
                {
                    continue;
                }
                let mut bundled = WandData::new(lhs, rhs, Vec::new());
                bundled.bundled_owner = true;
                for &id in selected.iter().rev() {
                    stratified.remove(id);
                }
                stratified.insert(selected[0], bundled);
            }
        }
        let mut wands = stratified;
        let resource_count = wands.len();
        wands.extend(resourceless);
        for wand in &mut wands {
            wand.pledges.clear();
        }
        for pledge in pledges {
            let selected = if let Some((argument, dereferences, opaque)) = pledge.source {
                let source = self.pledge_source(argument, dereferences, opaque, vcx)?;
                let matches = wands
                    .iter()
                    .enumerate()
                    .filter(|(_, wand)| wand.rhs.contains(&source))
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                let [selected] = matches.as_slice() else {
                    return Err(WandEncError::Unsupported(
                        "expiry selector must identify exactly one input restoration stage".into(),
                    ));
                };
                *selected
            } else {
                if wands.is_empty() {
                    return Err(WandEncError::Unsupported(
                        "expiry pledge has no borrowed input to restore".into(),
                    ));
                }
                if resource_count > 1 {
                    // An opaque payload introduces its own restoration stage,
                    // but does not make the enclosing borrow's expiry ambiguous.
                    // Preserve implicit pledges for a unique concrete lifetime
                    // stage (e.g. Holder<'a, T>::take -> &'a mut T). Independent
                    // concrete stages still need an explicit input selector.
                    let concrete = wands[..resource_count]
                        .iter()
                        .enumerate()
                        .filter(|(_, wand)| {
                            wand.rhs.iter().any(|source| {
                                let node: FunctionShapeNode<Generalized> = (*source).into();
                                let regions: IndexVec<_, GeneralizedLifetime<'vir>> =
                                    vcx.tcx().extract_regions(node.ty(sig));
                                matches!(
                                    regions.get(node.region_idx()),
                                    Some(GeneralizedLifetime::Region(_))
                                )
                            })
                        })
                        .map(|(index, _)| index)
                        .collect::<Vec<_>>();
                    let [selected] = concrete.as_slice() else {
                        return Err(WandEncError::Unsupported(
                            "pledges on a function whose result contains borrows that expire \
                             separately require an explicit input expiry selector"
                                .into(),
                        ));
                    };
                    *selected
                } else {
                    0
                }
            };
            wands[selected].pledges.push(*pledge);
        }
        Ok(wands
            .into_iter()
            .enumerate()
            .filter(|(index, wand)| *index < resource_count || !wand.pledges.is_empty())
            .map(|(_, wand)| wand)
            .collect())
    }

    /// Resolve the selector at the definition's signature. Dereferences choose
    /// a nested type, then its outer reference (or unique contained lifetime).
    /// Match the lifetime in the original input: repeated lifetimes are
    /// deduplicated, so counting dereferences is not a projection index.
    fn pledge_source(
        &self,
        argument: usize,
        dereferences: usize,
        opaque: bool,
        vcx: &'vir vir::VirCtxt<'vir>,
    ) -> Result<WandRhsKey, WandEncError> {
        let sig = self.fn_sig(vcx, None);
        let Some(original) = argument
            .checked_sub(1)
            .and_then(|index| sig.inputs().get(index))
        else {
            return Err(WandEncError::Unsupported(
                "invalid expiry input selector".into(),
            ));
        };
        let mut selected = *original;
        for _ in 0..dereferences {
            let ty::TyKind::Ref(_, target, _) = selected.kind() else {
                return Err(WandEncError::Unsupported(
                    "expiry selector dereferences a non-reference input type".into(),
                ));
            };
            selected = *target;
        }
        let lifetime = if opaque {
            // `self[_]` names a collection's element resources. Vec and its
            // owned iterator also contain an allocator, which is not an element.
            let mut collection = selected;
            while let ty::TyKind::Ref(_, inner, _) = collection.kind() {
                collection = *inner;
            }
            if let ty::TyKind::Adt(_, args) = collection.kind()
                && matches!(
                    RustTyDecomposition::from_ty(collection, self.g_params(vcx, None))
                        .ty
                        .special,
                    crate::encoders::ty::RustTySpecial::Vec
                        | crate::encoders::ty::RustTySpecial::VecIntoIter
                )
            {
                selected = args.type_at(0);
            }
            let regions: IndexVec<_, GeneralizedLifetime<'vir>> =
                vcx.tcx().extract_regions(selected);
            let groups = regions
                .iter()
                .copied()
                .filter(|region| matches!(region, GeneralizedLifetime::RegionsIn(_)))
                .collect::<Vec<_>>();
            match groups.as_slice() {
                [group] => *group,
                _ => {
                    return Err(WandEncError::Unsupported(
                        "expiry selector must identify a unique opaque input resource group".into(),
                    ));
                }
            }
        } else if let ty::TyKind::Ref(region, _, _) = selected.kind() {
            GeneralizedLifetime::Region((*region).into())
        } else {
            let regions: IndexVec<_, GeneralizedLifetime<'vir>> =
                vcx.tcx().extract_regions(selected);
            let concrete = regions
                .iter()
                .copied()
                .filter(|region| matches!(region, GeneralizedLifetime::Region(_)))
                .collect::<Vec<_>>();
            match concrete.as_slice() {
                [region] => *region,
                [] if regions.len() == 1 => *regions.iter().next().unwrap(),
                _ => {
                    return Err(WandEncError::Unsupported(
                        "expiry selector's input type must contain a unique borrowed lifetime"
                            .into(),
                    ));
                }
            }
        };
        let regions: IndexVec<_, GeneralizedLifetime<'vir>> = vcx.tcx().extract_regions(*original);
        self.inputs
            .iter()
            .copied()
            .find(|input| {
                input.mir_local().index() == argument
                    && regions.get(input.region_idx()) == Some(&lifetime)
            })
            .ok_or_else(|| {
                WandEncError::Unsupported(
                    "expiry selector does not identify a borrowed input resource".into(),
                )
            })
    }

    pub fn viper_wands(&self) -> Vec<WandData<'vir>> {
        self.wands.clone()
    }

    /// All lifetime projections in the arguments that are blocked by any of the
    /// lifetime projections in the function's result.
    pub fn blocked_inputs(&self) -> FxHashSet<FunctionShapeInput<Generalized>> {
        self.wands
            .iter()
            .flat_map(|wand| wand.rhs.iter().copied())
            .collect()
    }

    pub fn inputs(&self) -> impl Iterator<Item = FunctionShapeInput<Generalized>> + '_ {
        self.inputs.iter().copied()
    }

    pub fn outputs(&self) -> impl Iterator<Item = FunctionShapeOutput<Generalized>> + '_ {
        self.outputs.iter().copied()
    }
}

type OwnerCompositionArguments = (vir::ManyPSnap, vir::ManyTyVal);

/// Compose abstract histories in a checked method with immutable resource keys.
/// The shared continuation lowering gives this method two half continuations
/// per logical input and output, exactly as at its callers. The helper never
/// borrows the caller's current resource or assumes an intermediate identity.
pub(crate) struct OwnerCompositionEnc;

impl TaskEncoder for OwnerCompositionEnc {
    task_encoder::encoder_cache!(OwnerCompositionEnc);
    const ENCODER_NAME: &'static str = "owner continuation composition encoder";
    type TaskDescription<'vir> = ();
    type OutputFullDependency<'vir> = vir::MethodIdn<'vir, OwnerCompositionArguments>;
    type OutputFullLocal<'vir> = vir::Method<'vir>;

    fn task_to_key<'vir>(_task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {}

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        deps.require_ref::<crate::encoders::ty::resources::ResourceGroupEnc>(
            RustTyDecomposition::param(),
        )?;
        vir::with_vcx(|vcx| {
            let snapshots = vcx.alloc_slice(&[
                vcx.mk_local_decl("composition_current", vir::TYPE_PSNAP),
                vcx.mk_local_decl("composition_middle", vir::TYPE_PSNAP),
                vcx.mk_local_decl("composition_entry", vir::TYPE_PSNAP),
                vcx.mk_local_decl("composition_current_metadata", vir::TYPE_PSNAP),
                vcx.mk_local_decl("composition_middle_metadata", vir::TYPE_PSNAP),
                vcx.mk_local_decl("composition_entry_metadata", vir::TYPE_PSNAP),
            ]);
            let types = vcx.alloc_slice(&[
                vcx.mk_local_decl("composition_current_type", vir::TYPE_TYVAL),
                vcx.mk_local_decl("composition_middle_type", vir::TYPE_TYVAL),
                vcx.mk_local_decl("composition_entry_type", vir::TYPE_TYVAL),
            ]);
            let group = |index: usize| {
                crate::encoders::ty::resources::group(
                    vcx.mk_local_ex(snapshots[index]),
                    vcx.mk_local_ex(types[index]),
                    vcx.mk_local_ex(snapshots[index + 3]),
                )
            };
            let current = group(0);
            let middle = group(1);
            let entry = group(2);
            let recent = resource_restoration_wand(vcx, current, middle);
            let summary = resource_restoration_wand(vcx, middle, entry);
            let combined = resource_restoration_wand(vcx, current, entry);
            let statements = vcx.alloc_slice(&[vcx.mk_package_stmt(
                combined,
                vcx.alloc_slice(&direct_resource_restoration(
                    vcx,
                    current,
                    entry,
                    vec![vcx.mk_apply_stmt(recent), vcx.mk_apply_stmt(summary)],
                )),
            )]);
            let block = vcx.mk_cfg_block(
                &vir::CfgBlockLabelData::Start,
                &[],
                statements,
                &vir::TerminatorStmtGenData::Exit,
            );
            let method = vir::MethodIdn::<OwnerCompositionArguments>::new(
                vir::ViperIdent::new("owner_resource_compose"),
                (
                    vcx.alloc_slice(&[vir::TYPE_PSNAP; 6]),
                    vcx.alloc_slice(&[vir::TYPE_TYVAL; 3]),
                ),
            );
            Ok((
                vcx.mk_method(
                    method,
                    (snapshots, types),
                    &[],
                    vcx.alloc_slice(&[vcx.mk_wand_expr(recent), vcx.mk_wand_expr(summary)]),
                    vcx.alloc_slice(&[vcx.mk_wand_expr(combined)]),
                    Some(vcx.alloc_slice(&[block])),
                ),
                method,
            ))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for method in Self::all_outputs_local_no_errors(program) {
            program.add_method(method);
        }
    }
}

/// The helper covers one whole opaque group with complete resource keys.
/// Conditional, indexed and concrete groups keep their existing checked proof.
fn opaque_composition_keys<'vir>(
    current: vir::ExprBool<'vir>,
    middle: vir::ExprBool<'vir>,
    entry: vir::ExprBool<'vir>,
) -> Option<([vir::ExprPSnap<'vir>; 6], [vir::ExprTyVal<'vir>; 3])> {
    fn key(expr: vir::ExprBool<'_>) -> Option<&[vir::ExprDyn<'_>]> {
        let vir::ExprKindData::PredicateApp(app) = expr.kind else {
            return None;
        };
        (app.target == "indirect_resources"
            && app.args.len() == 3
            && app.perm.is_none_or(|p| {
                matches!(p.kind, vir::ExprKindData::Const(vir::ConstData::FullPerm))
            }))
        .then_some(app.args)
    }
    let (current, middle, entry) = (key(current)?, key(middle)?, key(entry)?);
    Some((
        [
            current[0].downcast_ty(),
            middle[0].downcast_ty(),
            entry[0].downcast_ty(),
            current[2].downcast_ty(),
            middle[2].downcast_ty(),
            entry[2].downcast_ty(),
        ],
        [
            current[1].downcast_ty(),
            middle[1].downcast_ty(),
            entry[1].downcast_ty(),
        ],
    ))
}

/// One checked continuation per stable generic input owner. Its snapshot
/// locals change only after composing an actual borrow-expiry continuation.
#[derive(Clone)]
pub(crate) struct OwnerSummary<'vir> {
    origin: mir::Local,
    group: usize,
    current: vir::ExprBool<'vir>,
    entry: vir::ExprBool<'vir>,
    bindings: Vec<(vir::LocalDeclSnap<'vir>, vir::ExprSnap<'vir>)>,
}

impl<'vir, E: TaskEncoder> ImpureEncVisitor<'vir, '_, E> {
    pub(crate) fn prepare_owner_summaries(&mut self) -> Vec<vir::Stmt<'vir>> {
        if self.spec_blocks.loop_specs.is_empty() {
            return Vec::new();
        }
        let vcx = self.vcx;
        let args = PledgeExpr::pledge_args(
            self.local_defs[mir::RETURN_PLACE].impure_snap,
            self.body
                .args_iter()
                .map(|arg| vcx.mk_old_expr(self.local_defs[arg].impure_snap)),
        );
        let inputs = self.wands.inputs.clone();
        let mut start = Vec::new();
        for node in inputs {
            if node.region_idx().index() == 0 {
                continue;
            }
            let shape: FunctionShapeNode<Generalized> = node.into();
            // Summarize each declared input group separately. Explicit prefix
            // restoration keeps its existing concrete-resource obligations;
            // projecting a callee group onto several actual input groups is
            // still handled by the boundary mapping, not an ordinal guess.
            let ty::TyKind::Ref(_, inner, ty::Mutability::Mut) =
                shape.ty(self.wands.fn_sig(vcx, None)).kind()
            else {
                continue;
            };
            if !matches!(inner.kind(), ty::TyKind::Param(..) | ty::TyKind::Alias(..))
                && self
                    .spec_blocks
                    .loop_specs
                    .values()
                    .any(|spec| !spec.restorations.is_empty())
            {
                continue;
            }
            let Some(entry) = self.wands.encode_predicates_for_function_shape_node(
                vcx,
                self.deps,
                node,
                None,
                Some(vir::OldLabel::None),
                None,
                |i| args[i],
            ) else {
                continue;
            };
            let captures = vcx.alloc(std::cell::RefCell::new(Vec::new()));
            let Some(current) = self.wands.encode_predicates_for_function_shape_node(
                vcx,
                self.deps,
                node,
                None,
                None,
                Some(captures),
                |i| args[i],
            ) else {
                continue;
            };
            let mut substitute = HandoffSubstitute(FxHashMap::default());
            let mut bindings = Vec::new();
            for (index, &(decl, value)) in captures.borrow().iter().enumerate() {
                let value = value.fold_with(&mut substitute).unwrap_or(value);
                let name = vir::vir_format!(
                    vcx,
                    "_owner_summary_{}_{}_{}",
                    node.mir_local().index(),
                    node.region_idx().index(),
                    index
                );
                let saved = vcx.mk_local_decl(name, value.ty());
                start.push(vcx.mk_local_decl_stmt(saved, None));
                start.push(vcx.mk_pure_assign_stmt(vcx.mk_local_ex(saved), value));
                bindings.push((saved, value));
                substitute
                    .0
                    .insert(decl.name, vcx.mk_local_ex(saved).as_dyn());
            }
            let current = current.fold_with(&mut substitute).unwrap_or(current);
            let reservation = vcx.mk_wand(vcx.mk_bool::<true>(), current);
            start.push(vcx.mk_package_stmt(reservation, &[]));
            start.push(vcx.mk_package_stmt(resource_restoration_wand(vcx, current, entry), &[]));
            start.push(vcx.mk_apply_stmt(reservation));
            self.owner_summaries.push(OwnerSummary {
                origin: node.mir_local(),
                group: node.region_idx().index(),
                current,
                entry,
                bindings,
            });
        }
        start
    }

    pub(crate) fn owner_summary_invariants(
        &mut self,
        loop_id: pcg::r#loop::LoopId,
        readable: impl Fn(mir::Local) -> bool,
    ) -> Vec<vir::ExprBool<'vir>> {
        let mut result = Vec::new();
        let mut snapshots = Vec::new();
        for summary in self.owner_summaries.clone() {
            result.push(self.vcx.mk_wand_expr(resource_restoration_wand(
                self.vcx,
                summary.current,
                summary.entry,
            )));
            if readable(summary.origin) {
                result.extend(summary.bindings.iter().map(|&(saved, value)| {
                    self.vcx.mk_eq_expr(self.vcx.mk_local_ex(saved), value)
                }));
            } else {
                // A whole alias may carry the input owner's resources through
                // the loop. Its logical input identity is ordinary ghost data,
                // independent of the permission heap. Reads are guarded by that
                // selection and backed by the alias's complete loop resource.
                let owner_ty = self.body.local_decls[summary.origin].ty;
                let ty::TyKind::Ref(_, owner_inner, ty::Mutability::Mut) = owner_ty.kind() else {
                    continue;
                };
                let expected = self
                    .vcx
                    .mk_const_expr(vir::ConstData::Int(summary.origin.index() as u128))
                    .downcast_ty::<vir::Int>();
                let mut aliases = self
                    .owner_origin_locals
                    .iter()
                    .filter_map(|(&local, &origin)| {
                        if !readable(local) {
                            return None;
                        }
                        let ty::TyKind::Ref(_, inner, ty::Mutability::Mut) =
                            self.body.local_decls[local].ty.kind()
                        else {
                            return None;
                        };
                        (inner == owner_inner)
                            .then_some((local, self.vcx.mk_eq_expr(origin, expected)))
                    })
                    .collect::<Vec<_>>();
                aliases.sort_by_key(|(local, _)| local.index());
                if aliases.is_empty() {
                    continue;
                }
                let selected = self.vcx.mk_disj(
                    &aliases
                        .into_iter()
                        .map(|(_, guard)| guard)
                        .collect::<Vec<_>>(),
                );
                let inactive = self.vcx.mk_eq_expr(selected, self.vcx.mk_bool::<false>());
                for &(saved, value) in &summary.bindings {
                    let current = self.vcx.mk_local_ex(saved);
                    let entry = self.new_tmp(current.ty());
                    snapshots.push((entry, current));
                    let observed = self.vcx.mk_eq_expr(current, value);
                    let unchanged = self.vcx.mk_eq_expr(current, entry);
                    result.push(vir::expr! { (selected) ==> (observed) });
                    // The unselected owner is framed outside the loop. Its
                    // summary variable still needs a pure equality because the
                    // other branch syntactically assigns that same variable.
                    result.push(vir::expr! { (inactive) ==> (unchanged) });
                }
            }
        }
        if !snapshots.is_empty() {
            self.loop_snapshot_frames
                .push(crate::encoders::mir_impure::LoopSnapshotFrame {
                    loop_id,
                    head: self.current_block_label.unwrap(),
                    snapshots,
                });
        }
        result
    }

    pub(crate) fn retain_owner_handoff(&mut self, restored: RestoredOwnerHandoff<'vir>) {
        // A package proof may use a summary but must not assign its enclosing
        // method's locals. Keep its new continuation local to that proof.
        let summary = if self.package_resource_stage.is_none() {
            self.owner_summaries
                .iter()
                .find(|summary| {
                    Some(summary.origin) == restored.origin && summary.group == restored.group
                })
                .cloned()
        } else {
            None
        };
        let Some(summary) = summary else {
            if self.package_resource_stage.is_none()
                && restored.origin.is_none()
                && let Some(selected) = restored.selection
            {
                let candidates = self
                    .owner_summaries
                    .iter()
                    .filter(|summary| summary.group == restored.group)
                    .cloned()
                    .collect::<Vec<_>>();
                if !candidates.is_empty() {
                    for summary in candidates {
                        let expected = self
                            .vcx
                            .mk_const_expr(vir::ConstData::Int(summary.origin.index() as u128))
                            .downcast_ty::<vir::Int>();
                        let guard = self.vcx.mk_eq_expr(selected, expected);
                        let script = self
                            .block(|visitor| {
                                visitor.compose_owner_handoff(&restored, &summary);
                                Ok::<_, std::convert::Infallible>(())
                            })
                            .unwrap();
                        self.stmt(
                            self.vcx
                                .mk_if_stmt(guard, self.vcx.alloc_slice(&script), &[]),
                        );
                    }
                    // An unknown origin cannot update a summary. Its existing
                    // checked snapshot and continuation invariants must still
                    // hold; no history with a future loop heap is assumed.
                    return;
                }
            }
            self.restored_owner_handoffs.push(restored);
            return;
        };
        self.compose_owner_handoff(&restored, &summary);
    }

    fn compose_owner_handoff(
        &mut self,
        restored: &RestoredOwnerHandoff<'vir>,
        summary: &OwnerSummary<'vir>,
    ) {
        let mut substitute = HandoffSubstitute(FxHashMap::default());
        let mut assignments = Vec::new();
        for &(saved, value) in &summary.bindings {
            let value = value.fold_with(&mut substitute).unwrap_or(value);
            let observed = self.new_tmp(value.ty());
            self.stmt(self.vcx.mk_pure_assign_stmt(observed, value));
            substitute.0.insert(saved.name, observed.as_dyn());
            assignments.push(
                self.vcx
                    .mk_pure_assign_stmt(self.vcx.mk_local_ex(saved), observed),
            );
        }
        let current = summary
            .current
            .fold_with(&mut substitute)
            .unwrap_or(summary.current);
        let vcx = self.vcx;
        let reservation = vcx.mk_wand(vcx.mk_bool::<true>(), current);
        self.stmt(vcx.mk_package_stmt(reservation, &[]));
        if let Some((snapshots, types)) =
            opaque_composition_keys(current, summary.current, summary.entry)
        {
            let compose = self.deps.require_dep::<OwnerCompositionEnc>(()).unwrap();
            self.stmt((compose)(vcx.alloc_slice(&snapshots), vcx.alloc_slice(&types)).alloc());
        } else {
            let compose = [
                vcx.mk_apply_stmt(restored.continuation),
                vcx.mk_apply_stmt(resource_restoration_wand(
                    vcx,
                    summary.current,
                    summary.entry,
                )),
            ];
            self.stmt(vcx.mk_package_stmt(
                resource_restoration_wand(vcx, current, summary.entry),
                vcx.alloc_slice(&direct_resource_restoration(
                    vcx,
                    current,
                    summary.entry,
                    compose.to_vec(),
                )),
            ));
        }
        self.stmt(vcx.mk_apply_stmt(reservation));
        for assignment in assignments {
            self.stmt(assignment);
        }
    }
}

impl<'vir, E: TaskEncoder> ImpureEncVisitor<'vir, '_, E> {
    /// The checked owner summary replaces the PCG's internal history wand
    /// for the same whole opaque input group. Keeping both would require
    /// duplicating its linear continuation and naming a future loop heap.
    /// This does not cover storage, projected owners, returned references,
    /// unsupported projected groups or a user-requested prefix-restoration formula.
    pub(crate) fn owner_summary_covers_loop_edge(
        &self,
        edge: &pcg::coupling::PcgCoupledEdgeKind<'vir>,
    ) -> bool {
        use pcg::{
            borrow_pcg::region_projection::PlaceOrConst,
            pcg::PcgNode,
            utils::{HasPlace, maybe_remote::MaybeRemotePlace},
        };
        let Some(head) = edge.loop_head() else {
            return false;
        };
        if self
            .spec_blocks
            .loop_specs
            .values()
            .any(|spec| spec.original_head_block == head && !spec.restorations.is_empty())
        {
            return false;
        }
        let covers = |projection: pcg::borrow_pcg::region_projection::LifetimeProjection<'vir>| {
            let PlaceOrConst::Place(place) = projection.base() else {
                return false;
            };
            let local = match place {
                MaybeRemotePlace::Local(place) if place.place().projection.is_empty() => {
                    place.place().local
                }
                MaybeRemotePlace::Remote(place) => place.assigned_local(),
                _ => return false,
            };
            self.owner_summaries.iter().any(|summary| {
                summary.origin == local && summary.group == projection.region_idx().index()
            })
        };
        let inputs = edge.inputs(self.pcg_ctxt());
        let outputs = edge.outputs();
        !inputs.is_empty()
            && !outputs.is_empty()
            && inputs.iter().all(|node| match **node {
                PcgNode::LifetimeProjection(projection) => covers(projection),
                _ => false,
            })
            && outputs.iter().all(|node| match **node {
                PcgNode::LifetimeProjection(projection) => covers(projection.into()),
                _ => false,
            })
    }
}
