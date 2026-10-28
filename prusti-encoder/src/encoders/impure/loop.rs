use rustc_hash::{FxHashMap, FxHashSet};

use pcg::{
    borrow_pcg::{
        edge::{abstraction::AbstractionEdge, kind::BorrowPcgEdgeKind},
        region_projection::{
            ExtractRegionsCtxt, LifetimeProjection, LifetimeProjectionLabel,
            PcgLifetimeProjectionBase, PcgLifetimeProjectionBaseLike,
        },
    },
    r#loop::{LoopId, PlaceUsages},
    pcg::{CapabilityKind, EvalStmtPhase, PcgNode},
    results::PcgBasicBlock,
    utils::{
        HasCompilerCtxt, HasPlace, Place, SnapshotLocation, maybe_old::MaybeLabelledPlace,
        maybe_remote::MaybeRemotePlace,
    },
};
use prusti_rustc_interface::{
    hir::Mutability,
    index::Idx,
    middle::{mir, ty},
};

use task_encoder::TaskEncoder;
use vir::Reify;

use crate::encoders::{
    EncodeResult, ImpureEncVisitor, TyUseImpureEnc,
    mir_impure::LoopReferenceFrame,
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
        loop_id: LoopId,
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

        // Singleton loop abstractions are packaged by `pcs_handle_edge` too.
        // Retain their wands just like grouped abstractions, or entering the
        // loop forgets how to return its borrowed resources to their owner.
        let edges: Vec<_> = state
            .borrow_pcg()
            .graph()
            .edges()
            .filter_map(|edge| match edge.kind() {
                BorrowPcgEdgeKind::Coupled(edge) => Some(edge.clone()),
                BorrowPcgEdgeKind::Abstraction(edge @ AbstractionEdge::Loop(_)) => {
                    Some(edge.clone().into_singleton_coupled_edge())
                }
                _ => None,
            })
            .collect();

        // A restoration target such as `*values` needs the shallow reference
        // value to identify its owner. Preserve that value's predicate without
        // also claiming the referent, which remains owned through the iterator.
        let mut reference_frames = FxHashSet::default();
        for edge in &edges {
            for input in edge.inputs(self.pcg_ctxt()) {
                let PcgNode::Place(MaybeLabelledPlace::Current(place)) = *input else {
                    continue;
                };
                for (prefix, projection) in place.iter_projections() {
                    let prefix: Place<'vir> = prefix.into();
                    if !matches!(projection, mir::ProjectionElem::Deref)
                        || !matches!(prefix.ty(ctxt).ty.kind(), ty::Ref(..))
                        || loop_invariant_place_capabilities
                            .get(&prefix)
                            .is_some_and(|capability| !capability.is_write())
                    {
                        continue;
                    }
                    if state
                        .computed_owned_capability(prefix, self.pcg_ctxt())
                        .is_some_and(|capability| !capability.is_write())
                    {
                        reference_frames.insert(prefix);
                    }
                }
            }
        }
        let mut snapshots = Vec::new();
        for place in reference_frames {
            let (encoded, snap, _, _) = self.encode_place_with_snap(place)?;
            let task = RustTyDecomposition::from_ty(place.ty(ctxt).ty, self.def_id);
            let ty = self.deps.require_dep::<TyUseImpureEnc>(task)?;
            inv.push(ty.ref_to_pred(self.vcx, encoded.expr.expect_predicate(), None));
            let name = vir::vir_format!(self.vcx, "_loop_ref{}", self.tmp_ctr);
            self.tmp_ctr += 1;
            let decl = self.vcx.mk_local_decl(name, snap.ty());
            self.stmt(self.vcx.mk_local_decl_stmt(decl, None));
            let saved = self.vcx.mk_local_ex(decl);
            inv.push(self.vcx.mk_eq_expr(snap, saved));
            snapshots.push((saved, snap));
        }
        if !snapshots.is_empty() {
            self.loop_reference_frames.push(LoopReferenceFrame {
                loop_id,
                head: self.current_block_label.unwrap(),
                snapshots,
            });
        }
        for edge in edges {
            let mut let_bind = WandOldOuter::LetBind(Vec::new());
            let (wand, _) = self.coupled_wand(&edge, &mut let_bind)?;
            // The empty resource wand is a tautology and does not need a
            // packaged wand instance at the loop boundary.
            if matches!(
                wand.lhs.kind,
                &vir::ExprKindData::Const(&vir::ConstData::Bool(true))
            ) && matches!(
                wand.rhs.kind,
                &vir::ExprKindData::Const(&vir::ConstData::Bool(true))
            ) {
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

    /// Initialise shallow reference snapshots on entry edges only. Capturing
    /// them on a backedge would erase the equality that the invariant must
    /// preserve. Pre-loop copies belong to the entry path even when their MIR
    /// block is syntactically inside the loop.
    pub(crate) fn capture_loop_reference_frames(&mut self) {
        let vcx = self.vcx;
        let analysis = self.fpcs_analysis.analysis().loop_analysis();
        for block in &mut self.encoded_blocks {
            let source = block.label.label;
            let in_loop = |loop_id: LoopId| match source {
                vir::CfgBlockLabelData::BasicBlock(bb)
                | vir::CfgBlockLabelData::BasicBlockTerminator(bb) => {
                    analysis.in_loop(mir::BasicBlock::from_usize(*bb), loop_id)
                }
                vir::CfgBlockLabelData::PreLoopBasicBlock(bb, pres)
                | vir::CfgBlockLabelData::PreLoopBasicBlockTerminator(bb, pres) => {
                    !pres.iter().any(|id| **id == loop_id.index())
                        && analysis.in_loop(mir::BasicBlock::from_usize(*bb), loop_id)
                }
                _ => false,
            };
            let capture = |target: vir::CfgBlockLabel<'vir>, existing: &'vir [vir::Stmt<'vir>]| {
                let extra = self
                    .loop_reference_frames
                    .iter()
                    .filter(|frame| frame.head == target && !in_loop(frame.loop_id))
                    .flat_map(|frame| frame.snapshots.iter())
                    .map(|(saved, snap)| vcx.mk_pure_assign_stmt(*saved, *snap))
                    .collect::<Vec<_>>();
                if extra.is_empty() {
                    existing
                } else {
                    vcx.alloc_slice(&existing.iter().copied().chain(extra).collect::<Vec<_>>())
                }
            };
            let (stmts, terminator) = match block.terminator {
                vir::TerminatorStmtData::Goto(target) => {
                    (capture(*target, block.stmts), block.terminator)
                }
                vir::TerminatorStmtData::GotoIf(branch) => {
                    let targets = branch
                        .targets
                        .iter()
                        .map(|target| {
                            vcx.mk_goto_if_target(
                                target.value,
                                target.label,
                                capture(target.label, target.statements),
                            )
                        })
                        .collect::<Vec<_>>();
                    (
                        block.stmts,
                        vcx.mk_goto_if_stmt(
                            branch.value,
                            vcx.alloc_slice(&targets),
                            branch.otherwise,
                            capture(branch.otherwise, branch.otherwise_statements),
                        ),
                    )
                }
                _ => (block.stmts, block.terminator),
            };
            *block = vcx.mk_cfg_block(source, block.label.invariants, stmts, terminator);
        }
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
        let heap_label = match r.label() {
            Some(LifetimeProjectionLabel::Location(at)) => Some(self.get_location_label(at)),
            _ => None,
        };
        let (place_snap, ty, _) = match place {
            PcgLifetimeProjectionBase::Place(MaybeRemotePlace::Local(
                MaybeLabelledPlace::Current(p),
            )) if heap_label.is_some() => {
                let (_, snap, ty, ty_out) = self.encode_place_with_snap(p)?;
                (self.vcx.maybe_apply_label(snap, heap_label), ty, ty_out)
            }
            PcgLifetimeProjectionBase::Place(MaybeRemotePlace::Local(
                MaybeLabelledPlace::Labelled(p),
            )) if matches!(r.label(), Some(LifetimeProjectionLabel::Location(at))
                if self.snapshot_precedes(at, p.at())) =>
            {
                // Moving the base later must not replace the earlier value
                // selected by its lifetime projection (e.g. an iterator's
                // tail before `next`). When the base was moved first, retain
                // its saved shallow value and use the projection's heap label
                // only for resources reached through that value.
                let (_, snap, ty, ty_out) = self.encode_place_with_snap(p.place())?;
                (self.vcx.maybe_apply_label(snap, heap_label), ty, ty_out)
            }
            PcgLifetimeProjectionBase::Place(p) => {
                self.encode_maybe_remote_place_snap(p, old_outer)?
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
        let first_block = first.location().block;
        let second_block = second.location().block;
        if first_block != second_block {
            return self
                .body
                .basic_blocks
                .dominators()
                .dominates(first_block, second_block);
        }
        // The encoder uses one `Before` label per MIR statement, independent
        // of the PCG evaluation phase. Ref reassignment has a separate label
        // later in that statement; `After` is at the end of the block.
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
