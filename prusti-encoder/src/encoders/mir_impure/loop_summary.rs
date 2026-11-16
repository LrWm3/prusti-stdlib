//! One iteration-local scalar restoration summary; no completed loan history.
//! The wand is always established by a checked package, never inhaled here.
use super::*;
use pcg::{
    borrow_pcg::region_projection::{LifetimeProjection, PcgLifetimeProjectionBaseLike},
    utils::maybe_remote::MaybeRemotePlace,
};

#[derive(Clone)]
pub(crate) struct LoopRestorationFrame<'vir> {
    pub loop_id: LoopId,
    pub span: Span,
    pub edge: PcgCoupledEdgeKind<'vir>,
    pub source: Place<'vir>,
    pub iterator: mir::Local,
    pub restoration: crate::encoders::mir_pure::MirPureEncOutput<'vir>,
    pub saved: FxHashMap<mir::Local, vir::ExprSnap<'vir>>,
}
/// One backedge step, with immutable old/new models and an actual native call.
/// The selected next continuation remains the permission authority.
#[derive(Clone, Copy)]
pub(crate) struct ScalarSummaryJoin<'vir> {
    pub call: mir::BasicBlock,
    before: vir::ExprSeq<'vir>,
    after: vir::ExprSeq<'vir>,
    span: Span,
}

struct RestorationSubstitution<'vir> {
    replacements: FxHashMap<&'static str, vir::ExprDyn<'vir>>,
    seen: FxHashSet<&'static str>,
    invalid: bool,
}

impl<'vir> Folder<'vir, (), !> for RestorationSubstitution<'vir> {
    fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
        if let vir::ExprKindData::Local(local) = expr.kind
            && local.name.starts_with("__prusti_restore_")
        {
            if let Some((&name, &replacement)) = self.replacements.get_key_value(local.name)
                && replacement.ty() == expr.ty()
            {
                self.seen.insert(name);
                return Some(replacement);
            }
            self.invalid = true;
            return None;
        }
        expr.super_fold_with(self)
    }
}

impl<'vir, 'enc, E: TaskEncoder> ImpureEncVisitor<'vir, 'enc, E> {
    pub(crate) fn scalar_summary_join(
        &mut self,
        edge: &PcgCoupledEdgeKind<'vir>,
        label: &'vir str,
    ) -> EncodeResult<'vir, Option<ScalarSummaryJoin<'vir>>, E> {
        let Some(frame) = self.restoration_frame_for_edge(edge, false)? else {
            return Ok(None);
        };
        let block = self.current_block.unwrap();
        if !self.loop_analysis().in_loop(block, frame.loop_id)
            || self
                .current_block_pres
                .as_ref()
                .is_some_and(|p| p.contains(&frame.loop_id.index()))
        {
            return Ok(None);
        }
        let tcx = self.vcx.tcx();
        let mut calls = Vec::new();
        for (at, data) in self.body.basic_blocks.iter_enumerated() {
            if self.loop_analysis().innermost_loop(at) != Some(frame.loop_id)
                || self.spec_blocks.spec_arms.blocks.contains(&at)
            {
                continue;
            }
            let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                continue;
            };
            let TyKind::FnDef(def_id, _) = *func.ty(self.body, tcx).kind() else {
                continue;
            };
            if tcx.item_name(def_id).as_str() != "next"
                || tcx.trait_of_assoc(def_id) != tcx.lang_items().iterator_trait()
            {
                continue;
            }
            let receiver = args
                .first()
                .and_then(|a| a.node.place())
                .and_then(|p| p.as_local());
            let direct = args.len() == 1 && receiver.is_some_and(|receiver| data.statements.iter().rev().any(|statement| matches!(&statement.kind,
                mir::StatementKind::Assign(box (destination, mir::Rvalue::Ref(_, _, source)))
                    if destination.as_local() == Some(receiver) && source.as_local() == Some(frame.iterator))));
            if !direct {
                return Err(self.unsupported_rvalue("body_restore_prefix requires one direct next call on its iterator per backedge".to_string(), frame.span));
            }
            calls.push(at);
        }
        let [call] = calls.as_slice() else {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires one direct next call on its iterator per backedge"
                    .to_string(),
                frame.span,
            ));
        };
        if !self.body.basic_blocks.dominators().dominates(*call, block) {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires next to precede each backedge".to_string(),
                frame.span,
            ));
        }
        let query = prusti_interface::environment::EnvQuery::new(tcx);
        let prefixes = frame.restoration.inputs.iter().copied().filter(|local| {
            let TyKind::Adt(ghost, args) = self.body.local_decls[*local].ty.kind() else { return false; };
            if !query.is_adt_in_crate(*ghost, "prusti_contracts") || ghost.non_enum_variant().name.as_str() != "Ghost" { return false; }
            matches!(args.type_at(0).kind(), TyKind::Adt(seq, _) if query.is_adt_in_crate(*seq, "prusti_contracts") && seq.non_enum_variant().name.as_str() == "Seq")
        }).collect::<Vec<_>>();
        let [prefix] = prefixes.as_slice() else {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires one captured completed sequence".to_string(),
                frame.span,
            ));
        };
        let field = self
            .ty_use_pure(self.body.local_decls[*prefix].ty)
            .expect_structlike()
            .fields[0];
        let before = field.read(frame.saved[prefix].downcast_ty()).downcast_ty();
        let current = self.encode_place_with_snap((*prefix).into())?.1;
        let after = field
            .read(
                self.vcx
                    .mk_local_labelled_old_expr(current, label)
                    .downcast_ty(),
            )
            .downcast_ty();
        Ok(Some(ScalarSummaryJoin {
            call: *call,
            before,
            after,
            span: frame.span,
        }))
    }

    /// Check the value premise while the checked next antecedent provides the
    /// actual expired yield. Keep every existing resource and package check.
    pub(crate) fn check_summary_yield(
        &mut self,
        join: ScalarSummaryJoin<'vir>,
        option_ty: ty::Ty<'vir>,
        option: vir::ExprSnap<'vir>,
        post_call: &'vir str,
    ) -> EncodeResult<'vir, (), E> {
        let option = self
            .vcx
            .mk_local_labelled_old_expr(option, post_call)
            .downcast_ty::<vir::CSnap>();
        let option_use = self.ty_use_pure(option_ty);
        let enum_ = option_use.expect_enumlike();
        let some = &enum_.variants[1];
        let is_some = self
            .vcx
            .mk_eq_expr(enum_.snap_to_discr_snap(option), some.data.discr);
        let reference = some.inner.fields[0].read(option);
        let TyKind::Adt(_, args) = option_ty.kind() else {
            unreachable!()
        };
        let reference_ty = args.type_at(0);
        let reference_use = self.ty_use_pure(reference_ty);
        let address = reference_use
            .expect_mutref()
            .deref_access(reference.downcast_ty());
        let reference_task = RustTyDecomposition::from_ty(reference_ty, self.def_id);
        let inner = reference_task
            .ty
            .expect_mutref()
            .referent
            .decompose_context(reference_task.ty.params, reference_task.args);
        let owner = self.deps.require_dep::<TyUseImpureEnc>(inner)?;
        let actual = owner.ref_to_snap(address).downcast_ty::<vir::PSnap>();
        let extended = self.vcx.mk_seq_concat_expr(
            join.before,
            self.vcx
                .mk_seq_literal_expr(self.vcx.alloc_slice(&[actual]), vir::TYPE_PSNAP),
        );
        let equal = self.vcx.mk_eq_expr(join.after, extended);
        for obligation in [is_some, equal] {
            let statement = self.vcx.with_span(join.span, |vcx| {
                vcx.handle_error("assert.failed:assertion.false", move |_| {
                    Some(vec![PrustiError::verification(
                        "loop restoration might not hold",
                        join.span.into(),
                    )])
                });
                vcx.mk_assert_stmt(obligation)
            });
            self.stmt(statement);
        }
        Ok(())
    }

    /// Recover the immutable logical model's actual initialization, rather
    /// than referring to the borrowed owner in a source-level invariant.
    /// The resulting equality is checked on loop entry and every backedge.
    pub(crate) fn immutable_model_origin(&self, local: mir::Local) -> Option<vir::OldLabel<'vir>> {
        let ty = self.body.local_decls[local].ty;
        if self.body.local_decls[local].mutability != ty::Mutability::Not {
            return None;
        }
        let tcx = self.vcx.tcx();
        let query = prusti_interface::environment::EnvQuery::new(tcx);
        let TyKind::Adt(ghost, args) = ty.kind() else {
            return None;
        };
        if !query.is_adt_in_crate(*ghost, "prusti_contracts")
            || ghost.non_enum_variant().name.as_str() != "Ghost"
        {
            return None;
        }
        let TyKind::Adt(seq, _) = args.type_at(0).kind() else {
            return None;
        };
        if !query.is_adt_in_crate(*seq, "prusti_contracts")
            || seq.non_enum_variant().name.as_str() != "Seq"
        {
            return None;
        }
        let mut initialization = None;
        for (block, data) in self.body.basic_blocks.iter_enumerated() {
            if self.spec_blocks.spec_arms.blocks.contains(&block) {
                continue;
            }
            if data.statements.iter().any(|statement| {
                matches!(&statement.kind,
                mir::StatementKind::Assign(box (destination, _)) if destination.local == local)
            }) {
                return None;
            }
            let mir::TerminatorKind::Call {
                func, destination, ..
            } = &data.terminator().kind
            else {
                continue;
            };
            if destination.as_local() != Some(local) {
                continue;
            }
            let func_ty = func.ty(self.body, tcx);
            if !matches!(func_ty.kind(), TyKind::FnDef(..)) {
                return None;
            }
            let (def_id, args) = RustSignature::get_def_id_and_caller_substs(func_ty);
            if !matches!(
                PrustiBuiltin::new(def_id, self.gargs(args)),
                Some(PrustiBuiltin::Ghost(
                    crate::encoders::builtin::GhostOp::Call
                ))
            ) || initialization.replace(block).is_some()
            {
                return None;
            }
        }
        let block = initialization?;
        if self.loop_analysis().loops(block).next().is_some() {
            return None;
        }
        // Raw MIR also contains ghost_erased(), which bypasses the capture.
        // Check dominance in the very same CFG that Prusti encodes: a spec
        // switch has exactly its selected live successor, not both MIR arms.
        let head = self.current_block?;
        let mut queue = std::collections::VecDeque::from([mir::START_BLOCK]);
        let mut visited = FxHashSet::default();
        while let Some(at) = queue.pop_front() {
            if at == block || self.spec_blocks.spec_arms.blocks.contains(&at) || !visited.insert(at)
            {
                continue;
            }
            if at == head {
                return None;
            }
            if let Some(&live) = self.spec_blocks.spec_arms.switches.get(&at) {
                queue.push_back(live);
            } else {
                queue.extend(self.body.basic_blocks.successors(at));
            }
        }
        Some(self.get_location_label(SnapshotLocation::After(block)))
    }

    fn restoration_source_for_edge(
        &self,
        edge: &PcgCoupledEdgeKind<'vir>,
        iterator: mir::Local,
    ) -> Option<Place<'vir>> {
        let outputs = edge.outputs();
        if outputs.is_empty()
            || !outputs.iter().all(|output| {
                let PcgNode::LifetimeProjection(output) = **output else {
                    return false;
                };
                matches!(output.base().to_pcg_lifetime_projection_base(),
                PlaceOrConst::Place(MaybeRemotePlace::Local(place))
                    if place.place() == iterator.into())
            })
        {
            return None;
        }
        let inputs = edge.inputs(self.pcg_ctxt());
        let storage = inputs
            .iter()
            .filter_map(|input| match **input {
                PcgNode::Place(MaybeLabelledPlace::Current(source)) => Some(source),
                _ => None,
            })
            .collect::<Vec<_>>();
        let [source] = storage.as_slice() else {
            return None;
        };
        if !matches!(source.projection.as_ref(), [mir::ProjectionElem::Deref])
            || !matches!(source.ty(self.pcg_ctxt()).ty.kind(), TyKind::Slice(_))
            || !matches!(
                self.body.local_decls[source.local].ty.kind(),
                TyKind::Ref(_, _, ty::Mutability::Mut)
            )
        {
            return None;
        }
        inputs
            .iter()
            .all(|input| match **input {
                PcgNode::Place(MaybeLabelledPlace::Current(place)) => place == *source,
                PcgNode::LifetimeProjection(input) => {
                    matches!(input.base().to_pcg_lifetime_projection_base(),
                PlaceOrConst::Place(MaybeRemotePlace::Local(place)) if *source == place.place())
                }
                _ => false,
            })
            .then_some(*source)
    }

    pub(crate) fn select_restoration_edge(
        &mut self,
        original_head: mir::BasicBlock,
        spec_block: mir::BasicBlock,
        span: Span,
        cfpcs: &PcgBasicBlock<'_, 'vir>,
    ) -> EncodeResult<
        'vir,
        (
            crate::encoders::mir_pure::MirPureEncOutput<'vir>,
            PcgCoupledEdgeKind<'vir>,
            Place<'vir>,
            mir::Local,
        ),
        E,
    > {
        let restoration = self.deps.require_dep::<MirPureEnc>(MirPureEncTask {
            encoding_depth: 0,
            parent_def_id: self.def_id,
            gargs: GParams::from(self.def_id).identity_args(),
            kind: PureKind::RestorationBlock(spec_block),
        })?;
        let iterator_locals = restoration
            .inputs
            .iter()
            .copied()
            .filter(|local| {
                RustTyDecomposition::from_ty(self.body.local_decls[*local].ty, self.context())
                    .ty
                    .special
                    == RustTySpecial::SliceIterMut
            })
            .collect::<Vec<_>>();
        if iterator_locals.len() != 1 {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires a unique local mutable slice iterator".to_string(),
                span,
            ));
        }
        let iterator = iterator_locals[0];
        let state = &cfpcs.statements[0].states[EvalStmtPhase::PreOperands];
        let loop_edges = state
            .borrow_pcg()
            .graph()
            .edges()
            .filter_map(|edge| {
                let edge = match edge.kind() {
                    BorrowPcgEdgeKind::Abstraction(edge @ AbstractionEdge::Loop(_)) => {
                        edge.clone().into_singleton_coupled_edge()
                    }
                    BorrowPcgEdgeKind::Coupled(edge) => edge.clone(),
                    _ => return None,
                };
                if edge.loop_head() != Some(original_head) {
                    return None;
                }
                Some(edge)
            })
            .collect::<Vec<_>>();
        let matched_edges = loop_edges
            .iter()
            .filter_map(|edge| {
                self.restoration_source_for_edge(edge, iterator)
                    .map(|source| (edge.clone(), source))
            })
            .collect::<Vec<_>>();
        if matched_edges.len() != 1 {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires a unique matching iterator restoration edge"
                    .to_string(),
                span,
            ));
        }
        let source = matched_edges[0].1;
        // Separate loop edges can retain another live borrower of this owner.
        // Selecting only its iterator edge would silently ignore that overlap.
        let overlaps = loop_edges.iter().any(|edge| {
            edge != &matched_edges[0].0
                && edge
                    .inputs(self.pcg_ctxt())
                    .iter()
                    .any(|input| match **input {
                        PcgNode::Place(place) => place.place().is_prefix_of(source),
                        PcgNode::LifetimeProjection(input) => matches!(
                        input.base(),
                        PlaceOrConst::Place(MaybeRemotePlace::Local(place))
                            if place.place().is_prefix_of(source)),
                    })
        });
        if overlaps {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires a unique matching iterator restoration edge"
                    .to_string(),
                span,
            ));
        }
        let slice_ty = source.ty(self.pcg_ctxt()).ty;
        let TyKind::Slice(element) = slice_ty.kind() else {
            unreachable!()
        };
        if !matches!(
            element.kind(),
            TyKind::Bool | TyKind::Char | TyKind::Int(_) | TyKind::Uint(_)
        ) {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix currently requires scalar elements".to_string(),
                span,
            ));
        }
        let iterator_ty = self.body.local_decls[iterator].ty;
        let TyKind::Adt(_, iterator_args) = iterator_ty.kind() else {
            unreachable!()
        };
        let TyKind::Slice(element_ty) = slice_ty.kind() else {
            unreachable!()
        };
        // A reborrow gives references inside the iterator fresh MIR regions.
        // The PCG edges retain those lifetime obligations separately; this
        // check compares only the element representation used by the summary.
        let tcx = self.vcx.tcx();
        if tcx.erase_and_anonymize_regions(*element_ty)
            != tcx.erase_and_anonymize_regions(iterator_args.type_at(1))
        {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires matching source and iterator element types"
                    .to_string(),
                span,
            ));
        }
        Ok((
            restoration,
            matched_edges.into_iter().next().unwrap().0,
            source,
            iterator,
        ))
    }

    fn restoration_has_prior_next(
        &self,
        loop_id: LoopId,
        original_head: mir::BasicBlock,
        annotation_head: mir::BasicBlock,
    ) -> bool {
        let tcx = self.vcx.tcx();
        let mut queue = std::collections::VecDeque::from([original_head]);
        let mut visited = FxHashSet::default();
        while let Some(block) = queue.pop_front() {
            if block == annotation_head
                || !self.loop_analysis().in_loop(block, loop_id)
                || !visited.insert(block)
            {
                continue;
            }
            let terminator = self.body[block].terminator();
            if let mir::TerminatorKind::Call { func, .. } = &terminator.kind
                && let ty::FnDef(def_id, _) = *func.ty(self.body, tcx).kind()
                && tcx.item_name(def_id).as_str() == "next"
                && tcx.trait_of_assoc(def_id) == tcx.lang_items().iterator_trait()
            {
                return true;
            }
            if let Some(&live_target) = self.spec_blocks.spec_arms.switches.get(&block) {
                queue.push_back(live_target);
            } else {
                queue.extend(
                    self.body
                        .basic_blocks
                        .successors(block)
                        .filter(|successor| !self.spec_blocks.spec_arms.blocks.contains(successor)),
                );
            }
        }
        false
    }

    pub(crate) fn prepare_restoration_frame(
        &mut self,
        original_head: mir::BasicBlock,
        annotation_head: mir::BasicBlock,
        cfpcs: &PcgBasicBlock<'_, 'vir>,
        loop_id: LoopId,
        spec_block: mir::BasicBlock,
        span: Span,
    ) -> EncodeResult<'vir, LoopRestorationFrame<'vir>, E> {
        if self.restoration_has_prior_next(loop_id, original_head, annotation_head) {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires the annotation before iterator advancement"
                    .to_string(),
                span,
            ));
        }
        if let Some(frame) = self.loop_restoration_frames.get(&original_head) {
            return Ok(frame.clone());
        }
        let (restoration, edge, source, iterator) =
            self.select_restoration_edge(original_head, spec_block, span, cfpcs)?;
        let inputs = restoration
            .inputs
            .iter()
            .copied()
            .chain([source.local, iterator])
            .collect::<FxHashSet<_>>();
        let mut saved = FxHashMap::default();
        for local in inputs.into_iter().sorted_by_key(|local| local.index()) {
            let ty = self.ty_use_pure(self.body.local_decls[local].ty).snapshot;
            let name = vir::vir_format!(
                self.vcx,
                "_loop_restore{}_{}",
                original_head.index(),
                local.index()
            );
            let decl = self.vcx.mk_local_decl(name, ty);
            self.stmt(self.vcx.mk_local_decl_stmt(decl, None));
            saved.insert(local, self.vcx.mk_local_ex(decl));
        }
        let frame = LoopRestorationFrame {
            loop_id,
            span,
            edge,
            source,
            iterator,
            restoration,
            saved,
        };
        self.loop_restoration_frames
            .insert(original_head, frame.clone());
        Ok(frame)
    }

    pub(crate) fn capture_restoration_frame(
        &mut self,
        frame: &LoopRestorationFrame<'vir>,
    ) -> EncodeResult<'vir, (), E> {
        for (local, saved) in frame.saved.iter().sorted_by_key(|(local, _)| local.index()) {
            let current = self.encode_place_with_snap((*local).into())?.1;
            self.stmt(self.vcx.mk_pure_assign_stmt(*saved, current));
            // The completed model is data, not an owner permission. Make its
            // total specification lookups available to native slice indexing
            // via a checked rule over this particular immutable value.
            let ty = self.body.local_decls[*local].ty;
            let tcx = self.vcx.tcx();
            let query = prusti_interface::environment::EnvQuery::new(tcx);
            if let TyKind::Adt(ghost, args) = ty.kind()
                && query.is_adt_in_crate(*ghost, "prusti_contracts")
                && ghost.non_enum_variant().name.as_str() == "Ghost"
                && let TyKind::Adt(seq, _) = args.type_at(0).kind()
                && query.is_adt_in_crate(*seq, "prusti_contracts")
                && seq.non_enum_variant().name.as_str() == "Seq"
            {
                let model =
                    self.ty_use_pure(ty).expect_structlike().fields[0].read(saved.downcast_ty());
                let bridge = self
                    .deps
                    .require_dep::<crate::encoders::builtin::SeqNativeLookupLemmaEnc>(())?;
                self.stmt(bridge(model.downcast_ty()).alloc());
            }
        }
        Ok(())
    }

    fn restoration_frame_for_edge(
        &mut self,
        edge: &PcgCoupledEdgeKind<'vir>,
        inside_script: bool,
    ) -> EncodeResult<'vir, Option<LoopRestorationFrame<'vir>>, E> {
        let Some(original_head) = edge.loop_head() else {
            return Ok(None);
        };
        let Some((head, loop_id, annotations)) = self
            .spec_blocks
            .loop_specs
            .values()
            .find(|spec| spec.original_head_block == original_head && !spec.restorations.is_empty())
            .map(|spec| (spec.head_block, spec.loop_id, spec.restorations.clone()))
        else {
            return Ok(None);
        };
        let (spec_block, span) = annotations[0];
        if annotations.len() != 1 {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires exactly one restoration annotation per loop"
                    .to_string(),
                span,
            ));
        }
        if inside_script && !self.loop_restoration_frames.contains_key(&original_head) {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires an entry package outside a proof script".to_string(),
                span,
            ));
        }
        let frame = if let Some(frame) = self.loop_restoration_frames.get(&original_head) {
            frame.clone()
        } else {
            let Ok(Some(cfpcs)) = self.fpcs_analysis.get_all_for_bb(head) else {
                return Err(self.unsupported_rvalue(
                    "body_restore_prefix requires a reachable loop invariant".to_string(),
                    span,
                ));
            };
            self.prepare_restoration_frame(original_head, head, &cfpcs, loop_id, spec_block, span)?
        };
        let selected_outputs = frame.edge.outputs();
        let outputs = edge.outputs();
        let mentions_iterator = outputs.iter().any(|output| {
            let PcgNode::LifetimeProjection(output) = **output else {
                return false;
            };
            matches!(output.base().to_pcg_lifetime_projection_base(),
                PlaceOrConst::Place(MaybeRemotePlace::Local(place))
                    if place.place() == frame.iterator.into())
        });
        if !mentions_iterator {
            return Ok(None);
        }
        let input_regions = |edge: &PcgCoupledEdgeKind<'vir>| {
            edge.inputs(self.pcg_ctxt())
                .iter()
                .filter_map(|input| match **input {
                    PcgNode::LifetimeProjection(input) => Some(input.region_idx()),
                    _ => None,
                })
                .collect::<FxHashSet<_>>()
        };
        let output_regions = outputs
            .iter()
            .map(|output| output.expect_lifetime_projection().region_idx())
            .collect::<FxHashSet<_>>();
        let selected_regions = selected_outputs
            .iter()
            .map(|output| output.expect_lifetime_projection().region_idx())
            .collect::<FxHashSet<_>>();
        if self.restoration_source_for_edge(edge, frame.iterator) != Some(frame.source)
            || input_regions(edge) != input_regions(&frame.edge)
            || output_regions != selected_regions
        {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires a stable matching iterator restoration edge"
                    .to_string(),
                span,
            ));
        }
        Ok(Some(frame))
    }

    pub(crate) fn with_restoration_wand(
        &mut self,
        wand: vir::Wand<'vir>,
        edge: &PcgCoupledEdgeKind<'vir>,
        old_outer: &mut WandOldOuter<'vir>,
        use_saved: bool,
        inside_script: bool,
    ) -> EncodeResult<'vir, vir::Wand<'vir>, E> {
        let Some(frame) = self.restoration_frame_for_edge(edge, inside_script)? else {
            return Ok(wand);
        };
        let locals = if use_saved {
            if self
                .current_block_pres
                .as_ref()
                .is_some_and(|pres| pres.contains(&frame.loop_id.index()))
            {
                return Err(self.unsupported_rvalue(
                    "body_restore_prefix requires the invariant before applying its loop wand"
                        .to_string(),
                    frame.span,
                ));
            }
            frame.saved.clone()
        } else {
            let mut locals = FxHashMap::default();
            for local in frame
                .saved
                .keys()
                .copied()
                .sorted_by_key(|local| local.index())
            {
                let snap = self.encode_place_with_snap(local.into())?.1;
                locals.insert(local, self.mk_wand_outer(snap, old_outer));
            }
            locals
        };
        let relation = self.encode_restoration_relation(
            &frame.restoration,
            frame.source,
            frame.iterator,
            locals,
            frame.span,
        )?;
        let lhs = if use_saved {
            // A `next` call can change the live tail even when it returns
            // `None`. Its expiry pledge restores the pre-call resource, so
            // apply the loop-head wand to the saved tail rather than the
            // iterator snapshot at the later drop or backedge.
            let mut resources = Vec::new();
            let mut outputs = edge.outputs();
            outputs.sort_by_key(|output| output.expect_lifetime_projection().region_idx());
            for output in outputs {
                let output = output.expect_lifetime_projection();
                // The saved iterator fixes the loop-head tail. Its resources
                // are supplied by the wand's current antecedent heap, not by
                // a later PCG label attached while advancing the iterator.
                resources.extend(self.encode_lifetime_projection_with_snapshot(
                    LifetimeProjection::from_index(output.base(), output.region_idx()),
                    old_outer,
                    Some(frame.saved[&frame.iterator]),
                )?);
            }
            self.vcx.mk_conj(&resources)
        } else {
            wand.lhs
        };
        Ok(self
            .vcx
            .mk_wand(lhs, self.vcx.mk_conj(&[wand.rhs, relation])))
    }

    pub(crate) fn encode_restoration_relation(
        &mut self,
        restoration: &crate::encoders::mir_pure::MirPureEncOutput<'vir>,
        source: Place<'vir>,
        iterator: mir::Local,
        locals: FxHashMap<mir::Local, vir::ExprSnap<'vir>>,
        span: Span,
    ) -> EncodeResult<'vir, vir::ExprBool<'vir>, E> {
        let Some((&source_snap, &iterator_snap)) = locals
            .get(&source.local)
            .zip(locals.get(&iterator))
            .filter(|_| {
                restoration
                    .inputs
                    .iter()
                    .all(|local| locals.contains_key(local))
            })
        else {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix requires snapshots for all restoration inputs".to_string(),
                span,
            ));
        };
        let slice_ty = source.ty(self.pcg_ctxt()).ty;
        let iterator_ty = self.body.local_decls[iterator].ty;
        let TyKind::Adt(_, iterator_args) = iterator_ty.kind() else {
            unreachable!()
        };
        let source_use = self.ty_use_pure(self.body.local_decls[source.local].ty);
        let source_ref = source_use.expect_mutref();
        let source_addr = source_ref.deref_access(source_snap.downcast_ty());
        let source_len = source_ref.metadata_access(source_snap.downcast_ty());
        let usize_use = self.ty_use_pure(self.vcx.tcx().types.usize);
        let source_len = usize_use
            .expect_primitive()
            .snap_to_prim(source_len.downcast_ty())
            .downcast_ty::<vir::Int>();
        let iterator_use = self.ty_use_pure(iterator_ty);
        let tail = iterator_use
            .expect_structlike()
            .fields
            .last()
            .unwrap()
            .read(iterator_snap.downcast_ty());
        let tail_ty = ty::Ty::new_mut_ref(self.vcx.tcx(), iterator_args.region_at(0), slice_ty);
        let tail_use = self.ty_use_pure(tail_ty);
        let tail_ref = tail_use.expect_mutref();
        let tail_addr = tail_ref.deref_access(tail.downcast_ty());
        let tail_len = tail_ref.metadata_access(tail.downcast_ty());
        let tail_len = usize_use
            .expect_primitive()
            .snap_to_prim(tail_len.downcast_ty())
            .downcast_ty::<vir::Int>();
        let relation = restoration
            .expr
            .reify(
                self.vcx,
                (
                    self.def_id,
                    self.vcx.alloc(locals),
                    vir::OldLabel::None,
                    None,
                ),
            )
            .downcast_ty::<vir::Bool>();
        let mut substitution = RestorationSubstitution {
            replacements: FxHashMap::from_iter([
                (
                    crate::encoders::mir_pure::RESTORE_SOURCE_ADDR,
                    source_addr.as_dyn(),
                ),
                (
                    crate::encoders::mir_pure::RESTORE_SOURCE_LEN,
                    source_len.as_dyn(),
                ),
                (
                    crate::encoders::mir_pure::RESTORE_TAIL_ADDR,
                    tail_addr.as_dyn(),
                ),
                (
                    crate::encoders::mir_pure::RESTORE_TAIL_LEN,
                    tail_len.as_dyn(),
                ),
            ]),
            seen: FxHashSet::default(),
            invalid: false,
        };
        let relation = relation.fold_with(&mut substitution).unwrap_or(relation);
        if substitution.invalid || substitution.seen.len() != substitution.replacements.len() {
            return Err(self.unsupported_rvalue(
                "body_restore_prefix could not bind the selected iterator restoration edge"
                    .to_string(),
                span,
            ));
        }
        Ok(relation)
    }
}
