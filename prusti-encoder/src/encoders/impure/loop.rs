use rustc_hash::{FxHashMap, FxHashSet};

use pcg::{
    borrow_pcg::{
        edge::{abstraction::AbstractionEdge, kind::BorrowPcgEdgeKind},
        region_projection::{
            ExtractRegionsCtxt, LifetimeProjection, LifetimeProjectionLabel,
            PcgLifetimeProjectionBase, PcgLifetimeProjectionBaseLike,
        },
    },
    r#loop::PlaceUsages,
    pcg::{CapabilityKind, EvalStmtPhase, PcgNode},
    results::PcgBasicBlock,
    utils::{
        HasCompilerCtxt, HasPlace, Place, SnapshotLocation, maybe_old::MaybeLabelledPlace,
        maybe_remote::MaybeRemotePlace,
    },
};
use prusti_rustc_interface::{
    hir::Mutability,
    middle::{mir, ty},
};

use task_encoder::TaskEncoder;
use vir::Reify;

use crate::encoders::{
    EncodeResult, ImpureEncVisitor, TyUseImpureEnc,
    ty::{
        RustTyDecomposition,
        indirect::{ExprInput, IndirectPredicatesEnc},
        use_impure::TyUseImpure,
    },
};

pub(crate) enum WandOldOuter<'vir> {
    LetBind(Vec<LetBind<'vir>>),
    Label(Option<&'vir str>),
}

impl<'vir: 'a, 'a, 'enc, E: TaskEncoder> ImpureEncVisitor<'vir, 'enc, E> {
    /// Calculate invariant at loop head
    pub(crate) fn get_loop_inv(
        &mut self,
        cfpcs: &PcgBasicBlock<'_, 'vir>,
        loop_place_usages: &PlaceUsages<'vir>,
        invariant_inputs: &FxHashSet<mir::Local>,
        ctxt: impl HasCompilerCtxt<'a, 'vir>,
    ) -> EncodeResult<
        'vir,
        (
            Vec<vir::ExprBool<'vir>>,
            FxHashMap<Place<'vir>, CapabilityKind>,
        ),
        E,
    > {
        let mut inv = Vec::new();
        let start = &cfpcs.statements[0];
        let state = &start.states[EvalStmtPhase::PreOperands];
        let mut loop_invariant_place_capabilities =
            cfpcs.loop_invariant_place_capabilities(loop_place_usages, ctxt);
        // PCG leaves a one-source/one-target loop abstraction uncoupled. Its
        // package/apply path still creates a wand, so the invariant must retain
        // that same wand just as it does for coupled loop edges.
        let restoration_edges = state
            .borrow_pcg()
            .graph()
            .edges()
            .filter_map(|edge| match edge.kind() {
                BorrowPcgEdgeKind::Abstraction(edge @ AbstractionEdge::Loop(_)) => {
                    Some(edge.clone().into_singleton_coupled_edge())
                }
                BorrowPcgEdgeKind::Coupled(edge) => Some(edge.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();

        // A restoration edge can refer to *arg even when the loop body never
        // reads arg itself. Retain the independently owned reference slot so
        // its address and metadata remain available to that edge. Its pointee
        // stays inside the restoration wand: adding its indirect predicates
        // here would incorrectly demand the borrowed owner's resources.
        let mut reference_slots = FxHashSet::default();
        for edge in &restoration_edges {
            for input in edge.inputs(self.pcg_ctxt()) {
                let PcgNode::Place(MaybeLabelledPlace::Current(place)) = *input else {
                    continue;
                };
                let local = place.local;
                if local.as_usize() == 0
                    || local.as_usize() > self.body.arg_count
                    || self.body.local_decls[local].mutability != Mutability::Not
                    || !matches!(place.projection.first(), Some(mir::ProjectionElem::Deref))
                    || !matches!(
                        self.body.local_decls[local].ty.kind(),
                        ty::Ref(_, _, Mutability::Mut)
                    )
                {
                    continue;
                }
                let slot = Place::from(local);
                if !loop_invariant_place_capabilities.contains_key(&slot)
                    && let Some(capability) = state.computed_owned_capability(slot, self.pcg_ctxt())
                    && !capability.is_write()
                {
                    loop_invariant_place_capabilities.insert(slot, capability);
                    reference_slots.insert(slot);
                }
            }
        }

        // The reference local can be absent from loop usages even when its
        // dereference is present. Preserve it when an invariant reads it and
        // the PCG has a readable owned capability at the loop boundary.
        for local in invariant_inputs {
            let place: Place<'vir> = (*local).into();
            if loop_invariant_place_capabilities.contains_key(&place)
                || !matches!(place.ty(ctxt).ty.kind(), ty::Ref(_, _, Mutability::Not))
            {
                continue;
            }
            if let Some(capability) = state.computed_owned_capability(place, self.pcg_ctxt())
                && !capability.is_write()
            {
                loop_invariant_place_capabilities.insert(place, capability);
            }
        }

        for (place, capability) in loop_invariant_place_capabilities.iter() {
            if capability.is_write() {
                continue; // No permissions are encoded for places with write capabilities currently
            }
            let (place_res, _snap, _, _) = self.encode_place_with_snap(*place)?;
            // Places behind a shared reference are observed through its
            // snapshot and do not have their own heap predicate to preserve.
            if place_res.expr.is_snapshot() {
                continue;
            }
            let ty = (*place).ty(self.pcg_ctxt());
            let task = RustTyDecomposition::from_ty(ty.ty, self.def_id);
            let ty_out = self.deps.require_dep::<TyUseImpureEnc>(task).unwrap();
            let pred = ty_out.ref_to_pred(self.vcx, place_res.expr.expect_predicate(), None);
            inv.push(pred);
            if reference_slots.contains(place) {
                // The argument binding is immutable. Check its stable identity
                // as an ordinary entry/backedge invariant, without reading the
                // borrowed pointee or assuming a value-restoration guarantee.
                inv.push(self.vcx.mk_eq_expr(_snap, self.vcx.mk_old_expr(_snap)));
                continue;
            }
            // gets all the lifetime projections of the place
            // for example, if we add an invariant for x and x is a reference, then we also want to add
            // an invariant for *x
            let projs = ctxt.extract_lifetime_projections(place.with_inherent_region(ctxt));
            for proj in projs {
                let indirect = self
                    .deps
                    .require_dep::<IndirectPredicatesEnc>(proj.with_base(task))
                    .unwrap();
                inv.extend(indirect.predicate_applications.iter().map(|p| {
                    p.reify(
                        self.vcx,
                        ExprInput::current(ty_out.ref_to_snap(place_res.expr.expect_predicate())),
                    )
                }));
            }
        }

        for edge in &restoration_edges {
            let mut let_bind = WandOldOuter::LetBind(Vec::new());
            let (wand, _) = self.coupled_wand(edge, &mut let_bind)?;
            // Keep empty-edge elimination consistent with package, footprint,
            // and apply emission in pcs_handle_wand.
            if super::wand::empty_resource_wand(wand) {
                continue;
            }
            let mut wand = self.vcx.mk_wand_expr(wand);
            let WandOldOuter::LetBind(let_bind) = let_bind else {
                unreachable!()
            };
            for lb in let_bind {
                wand = lb.map_or_else(
                    |(d, e)| self.vcx.mk_let_expr(d, e, wand),
                    |(d, e)| self.vcx.mk_let_expr(d, e, wand),
                );
            }
            inv.push(wand);
        }

        Ok((inv, loop_invariant_place_capabilities.into_iter().collect()))
    }

    pub(crate) fn encode_pcg_node<T: PcgLifetimeProjectionBaseLike<'vir>>(
        &mut self,
        node: &PcgNode<'vir, MaybeLabelledPlace<'vir>, T>,
        wand_rhs: &mut Vec<vir::ExprBool<'vir>>,
        old_outer: &mut WandOldOuter<'vir>,
    ) -> EncodeResult<'vir, (), E> {
        match node {
            PcgNode::Place(place) => {
                let p = place.place();
                let ty = (*p).ty(self.local_decls, self.vcx.tcx());
                let task = RustTyDecomposition::from_ty(ty.ty, self.def_id);
                let ty_out = self.deps.require_dep::<TyUseImpureEnc>(task).unwrap();
                let p = self.encode_place(p)?;
                let p = self.configure_old((*place).into(), p.expr.expect_predicate(), old_outer);

                let pred = ty_out.ref_to_pred(self.vcx, p, None);
                wand_rhs.push(pred);
            }
            PcgNode::LifetimeProjection(r) => {
                let exprs = self.encode_lifetime_projection(*r, old_outer)?;
                wand_rhs.extend(exprs);
            }
        }
        Ok(())
    }

    pub(super) fn encode_lifetime_projection<T: PcgLifetimeProjectionBaseLike<'vir>>(
        &mut self,
        r: LifetimeProjection<'vir, T>,
        old_outer: &mut WandOldOuter<'vir>,
    ) -> EncodeResult<'vir, Vec<vir::ExprBool<'vir>>, E> {
        let place = r.base().to_pcg_lifetime_projection_base();
        // The projection's history is separate from the lifetime of its local
        // storage. Advancing an iterator changes its tail while the previous
        // loop wand still names the pre-call tail. Preserve PCG's saved view,
        // including heap reads below mutable references in that view.
        let heap_label = match r.label() {
            Some(LifetimeProjectionLabel::Location(at)) => Some(self.get_location_label(at)),
            _ => None,
        };
        let (place_snap, ty) = match place {
            PcgLifetimeProjectionBase::Place(MaybeRemotePlace::Local(
                MaybeLabelledPlace::Current(p),
            )) if heap_label.is_some() => {
                let (_, snap, ty, _) = self.encode_place_with_snap(p)?;
                (self.vcx.maybe_apply_label(snap, heap_label), ty)
            }
            PcgLifetimeProjectionBase::Place(MaybeRemotePlace::Local(
                MaybeLabelledPlace::Labelled(p),
            )) if matches!(r.label(), Some(LifetimeProjectionLabel::Location(at))
                if self.snapshot_precedes(at, p.at())) =>
            {
                // A later move of the local does not replace an earlier tail.
                // If storage was moved first, preserve that saved shallow
                // value and use the projection label for its nested heap reads.
                let (_, snap, ty, _) = self.encode_place_with_snap(p.place())?;
                (self.vcx.maybe_apply_label(snap, heap_label), ty)
            }
            PcgLifetimeProjectionBase::Place(p) => {
                let (snap, ty, _) = self.encode_maybe_remote_place_snap(p, old_outer)?;
                (snap, ty)
            }
            PcgLifetimeProjectionBase::Const(c) => todo!("{c:?}"),
        };
        let ty = RustTyDecomposition::from_ty(ty.ty, self.def_id);
        let indirect = self
            .deps
            .require_dep::<IndirectPredicatesEnc>(r.with_base(ty))
            .unwrap();
        Ok(indirect
            .predicate_applications
            .into_iter()
            .map(|expr| {
                expr.reify(
                    self.vcx,
                    ExprInput {
                        heap_label,
                        ..ExprInput::current(place_snap)
                    },
                )
            })
            .collect::<Vec<_>>())
    }

    fn snapshot_precedes(&self, first: SnapshotLocation, second: SnapshotLocation) -> bool {
        if first.location().block != second.location().block {
            return self
                .body
                .basic_blocks
                .dominators()
                .dominates(first.location().block, second.location().block);
        }
        // Prusti has one Before label per MIR statement, then a distinct
        // reference-reassignment label, and After at the end of the block.
        let point = |at: SnapshotLocation| match at {
            SnapshotLocation::Loop(_) | SnapshotLocation::BeforeJoin(_) => (0, 0),
            SnapshotLocation::Before(at) => (at.location().statement_index, 0),
            SnapshotLocation::BeforeRefReassignment(at) => (at.statement_index, 1),
            SnapshotLocation::After(_) => (usize::MAX, 0),
        };
        point(first) <= point(second)
    }

    fn get_place(place: MaybeRemotePlace<'vir>) -> Place<'vir> {
        match place {
            MaybeRemotePlace::Local(MaybeLabelledPlace::Current(place)) => place,
            MaybeRemotePlace::Local(MaybeLabelledPlace::Labelled(place)) => place.place(),
            MaybeRemotePlace::Remote(r) => r.assigned_local().into(),
        }
    }

    fn encode_maybe_remote_place_snap(
        &mut self,
        place: MaybeRemotePlace<'vir>,
        old_outer: &mut WandOldOuter<'vir>,
    ) -> EncodeResult<'vir, (vir::ExprSnap<'vir>, mir::PlaceTy<'vir>, TyUseImpure<'vir>), E> {
        let p = Self::get_place(place);
        let (_, place_snap, ty, ty_out) = self.encode_place_with_snap(p)?;
        let place_snap = self.configure_old(place, place_snap, old_outer);
        Ok((place_snap, ty, ty_out))
    }

    fn configure_old<T: SnapOrRef>(
        &mut self,
        place: MaybeRemotePlace,
        expr: vir::Expr<'vir, T>,
        old_outer: &mut WandOldOuter<'vir>,
    ) -> vir::Expr<'vir, T> {
        match place {
            MaybeRemotePlace::Local(MaybeLabelledPlace::Current { .. }) => {
                self.mk_wand_outer(expr, old_outer)
            }
            MaybeRemotePlace::Local(MaybeLabelledPlace::Labelled(place)) => {
                let label = self.get_location_label(place.at());
                self.vcx.mk_old(expr, label)
            }
            MaybeRemotePlace::Remote(_) => self.vcx.mk_old_expr(expr),
        }
    }

    fn mk_wand_outer<T: SnapOrRef>(
        &mut self,
        expr: vir::Expr<'vir, T>,
        old_outer: &mut WandOldOuter<'vir>,
    ) -> vir::Expr<'vir, T> {
        match old_outer {
            WandOldOuter::LetBind(let_bind) => {
                let ident = vir::vir_format!(self.vcx, "_lb{}", let_bind.len());
                let decl = self.vcx.mk_local_decl(ident, expr.ty());
                let_bind.push(T::as_result(decl, expr));
                self.vcx.mk_local_ex(decl)
            }
            WandOldOuter::Label(label) => {
                let label = *label.get_or_insert_with(|| self.new_label("outer_package"));
                self.vcx.mk_local_labelled_old_expr(expr, label)
            }
        }
    }
}

type LetBind<'vir> = Result<
    (vir::LocalDeclSnap<'vir>, vir::ExprSnap<'vir>),
    (vir::LocalDeclRef<'vir>, vir::ExprRef<'vir>),
>;

trait SnapOrRef: vir::CompType {
    fn as_result<'vir>(d: vir::LocalDecl<'vir, Self>, e: vir::Expr<'vir, Self>) -> LetBind<'vir>;
}

impl SnapOrRef for vir::Snap {
    fn as_result<'vir>(d: vir::LocalDecl<'vir, Self>, e: vir::Expr<'vir, Self>) -> LetBind<'vir> {
        Ok((d, e))
    }
}

impl SnapOrRef for vir::Ref {
    fn as_result<'vir>(d: vir::LocalDecl<'vir, Self>, e: vir::Expr<'vir, Self>) -> LetBind<'vir> {
        Err((d, e))
    }
}
