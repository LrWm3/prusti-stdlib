use rustc_hash::{FxHashMap, FxHashSet};

use pcg::{
    borrow_pcg::{
        borrow_pcg_edge::BorrowPcgEdgeLike,
        borrow_pcg_expansion::BorrowPcgExpansion,
        edge::{
            abstraction::{AbstractionEdge, FunctionCallOrLoop},
            borrow_flow::{BorrowFlowEdge, BorrowFlowEdgeKind},
            kind::BorrowPcgEdgeKind,
        },
        region_projection::{
            ExtractRegionsCtxt, LifetimeProjection, LifetimeProjectionLabel,
            PcgLifetimeProjectionBase, PcgLifetimeProjectionBaseLike,
        },
    },
    coupling::PcgCoupledEdgeKind,
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
use vir::{CastType, Reify};

use crate::encoders::{
    EncodeResult, ImpureEncVisitor, TyUseImpureEnc, TyUsePureEnc,
    mir_impure::LoopSnapshotFrame,
    mir_shared::PureRvalueEnc,
    ty::{
        RustTyDecomposition, TySpecifics,
        generics::GArgsCastEnc,
        indirect::{ExprInput, IndirectPredicatesEnc},
        use_impure::TyUseImpure,
    },
};

pub(crate) enum WandOldOuter<'vir> {
    LetBind(Vec<LetBind<'vir>>),
    Label(Option<&'vir str>),
}

impl<'vir: 'a, 'a, 'enc, E: TaskEncoder> ImpureEncVisitor<'vir, 'enc, E> {
    fn resource_extent(
        &mut self,
        metadata: Option<vir::ExprSnap<'vir>>,
    ) -> EncodeResult<'vir, Option<vir::ExprInt<'vir>>, E> {
        metadata
            .map(|metadata| {
                let ty = RustTyDecomposition::from_ty(self.vcx.tcx().types.usize, self.def_id);
                let usize_ty = self.deps.require_dep::<TyUsePureEnc>(ty)?;
                Ok(usize_ty
                    .expect_primitive()
                    .snap_to_prim(metadata.downcast_ty())
                    .downcast_ty())
            })
            .transpose()
    }

    /// Read an element at a historical heap where the slot may still be
    /// folded into its owner. Every branch reads an actual owned predicate;
    /// permission inspection only selects its representation.
    fn indexed_element_snapshot(
        &mut self,
        owner: Place<'vir>,
        element: Place<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> EncodeResult<'vir, vir::ExprSnap<'vir>, E> {
        let (slot, _, _, child_impure) = self.encode_place_with_snap(element)?;
        let (parent, mut parent_snap, _, parent_impure) = self.encode_place_with_snap(owner)?;
        let slot_ref = parent_impure
            .expect_array()
            .ref_to_index_ref(parent.expr.expect_predicate(), index);
        let snap = child_impure.ref_to_snap(slot_ref);
        let owner_ty = RustTyDecomposition::from_ty(parent.ty.ty, self.def_id);
        let parent_pure = self.deps.require_dep::<TyUsePureEnc>(owner_ty)?;
        let has = |app| {
            let perm = self.vcx.mk_predicate_perm_expr(app);
            let none = self.vcx.mk_no_perm();
            vir::expr! { (none) < (perm) }
        };
        let mut parent_held =
            has(parent_impure.ref_to_pred_app(parent.expr.expect_predicate(), None));
        // A referent that has not been expanded is held in generic form.
        if let Some((prefix, mir::ProjectionElem::Deref)) = owner.last_projection() {
            let prefix_ty = prefix.ty(self.pcg_ctxt()).ty;
            let prefix_ty = RustTyDecomposition::from_ty(prefix_ty, self.def_id);
            if let TySpecifics::MutRef(data) = prefix_ty.ty.specifics {
                let generic = data
                    .referent
                    .decompose_context(prefix_ty.ty.params, prefix_ty.args);
                let generic = self.deps.require_dep::<TyUseImpureEnc>(generic)?;
                let caster = self
                    .deps
                    .require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
                        data.referent
                            .decompose_compare_normalize(prefix_ty.ty.params, prefix_ty.args),
                    )?;
                let fallback =
                    caster.cast_to_caller_ctx(generic.ref_to_snap(parent.expr.expect_predicate()));
                parent_held = self.vcx.mk_disj(&[
                    parent_held,
                    has(generic.ref_to_pred_app(parent.expr.expect_predicate(), None)),
                ]);
                parent_snap = self.vcx.mk_ternary_expr(
                    has(parent_impure.ref_to_pred_app(parent.expr.expect_predicate(), None)),
                    parent_snap,
                    fallback,
                );
            }
        }
        let parent_element = parent_pure
            .expect_array()
            .index(parent_snap.downcast_ty(), index);
        let TySpecifics::ArrayLike(ref data) = owner_ty.ty.specifics else {
            unreachable!()
        };
        let generic = data.decompose_context(owner_ty.ty.params, owner_ty.args);
        let generic = self.deps.require_dep::<TyUseImpureEnc>(generic)?;
        let caster = self
            .deps
            .require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
                data.decompose_compare_normalize(owner_ty.ty.params, owner_ty.args),
            )?;
        let generic_snap = caster.cast_to_caller_ctx(generic.ref_to_snap(slot_ref));
        let mut fallback = self.vcx.mk_ternary_expr(
            has(generic.ref_to_pred_app(slot_ref, None)),
            generic_snap,
            parent_element,
        );
        let child_ty = RustTyDecomposition::from_ty(slot.ty.ty, self.def_id);
        if let TySpecifics::StructLike(data) = &child_ty.ty.specifics {
            let pure = self.deps.require_dep::<TyUsePureEnc>(child_ty)?;
            let mut readable = Vec::new();
            let mut fields = Vec::new();
            for (field, access) in data
                .fields
                .iter()
                .zip(&child_impure.expect_structlike().fields)
            {
                let addr = access.field_ref(slot_ref);
                let concrete = self
                    .deps
                    .require_dep::<TyUseImpureEnc>(field.ty().decompose_normalize(child_ty.args))?;
                let generic = self.deps.require_dep::<TyUseImpureEnc>(
                    field
                        .ty()
                        .decompose_context(child_ty.ty.params, child_ty.args),
                )?;
                let caster = self
                    .deps
                    .require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
                        field
                            .ty()
                            .decompose_compare_normalize(child_ty.ty.params, child_ty.args),
                    )?;
                let concrete_held = has(concrete.ref_to_pred_app(addr, None));
                let generic_held = has(generic.ref_to_pred_app(addr, None));
                readable.push(self.vcx.mk_disj(&[concrete_held, generic_held]));
                fields.push(self.vcx.mk_ternary_expr(
                    concrete_held,
                    concrete.ref_to_snap(addr),
                    caster.cast_to_caller_ctx(generic.ref_to_snap(addr)),
                ));
            }
            let expanded = pure
                .expect_structlike()
                .field_snaps_to_snap(fields)
                .upcast_ty();
            fallback = self
                .vcx
                .mk_ternary_expr(self.vcx.mk_conj(&readable), expanded, fallback);
        }
        let expanded = self.vcx.mk_ternary_expr(
            has(child_impure.ref_to_pred_app(slot_ref, None)),
            snap,
            fallback,
        );
        // At a heap predating the expansion the owner supplies the element's
        // identity. Prefer that snapshot over any independently held predicate
        // whose address might coincide with the not-yet-exposed slot.
        Ok(self
            .vcx
            .mk_ternary_expr(parent_held, parent_element, expanded))
    }

    pub(crate) fn repack_array_aggregate(
        &mut self,
        flow: &BorrowFlowEdge<'vir>,
        fold: bool,
        label: Option<&'vir str>,
    ) -> EncodeResult<'vir, (), E> {
        let BorrowFlowEdgeKind::Aggregate { field_idx, .. } = flow.kind() else {
            return Ok(());
        };
        let target = flow.short();
        let owner = target.base().place();
        let ty = owner.ty(self.pcg_ctxt()).ty;
        if !matches!(ty.kind(), ty::Array(..)) {
            return Ok(());
        }
        // Collapsing an owned place also uses Aggregate edges. Its indexed
        // lifetime expansion handles those resources; only an aggregate
        // assignment transfers resources from independent operands here.
        if let PcgLifetimeProjectionBase::Place(source) = flow.long().base()
            && Self::get_place(source).prefix_place() == Some(owner)
        {
            return Ok(());
        }
        let task = RustTyDecomposition::from_ty(ty, self.def_id);
        let indirect = self
            .deps
            .require_dep::<IndirectPredicatesEnc>(target.with_base(task))?;
        let Some(array) = indirect.array else {
            return Ok(());
        };
        let mut old_outer = WandOldOuter::Label(label);
        let index = self
            .vcx
            .mk_const_expr(vir::ConstData::Int(field_idx as u128))
            .downcast_ty();
        let indexed = self.vcx.alloc(array.indexed.clone());
        let resources = self.encode_lifetime_projection_resources(
            flow.long(),
            &mut old_outer,
            None,
            Some((indexed, index, true)),
        )?;
        let resources = self.vcx.mk_conj(&resources);
        let stmts = array
            .indexed
            .operations(resources, index, fold)
            .map_err(|reason| self.unsupported_rvalue(reason.to_string(), self.current_span()))?;
        for stmt in stmts {
            self.stmt(stmt);
        }
        Ok(())
    }

    /// Moving a reference-bearing value into an indexed slot also transfers
    /// its referent resources, even when the slot is never read afterwards.
    pub(crate) fn repack_array_assignment(
        &mut self,
        flow: &BorrowFlowEdge<'vir>,
        fold: bool,
        label: Option<&'vir str>,
    ) -> EncodeResult<'vir, (), E> {
        let target = flow.short();
        let element = target.base().place();
        let Some((owner, projection)) = element.last_projection() else {
            return Ok(());
        };
        let ty = owner.ty(self.pcg_ctxt()).ty;
        if !matches!(ty.kind(), ty::Array(..) | ty::Slice(..)) {
            return Ok(());
        }
        let Ok(guide) = projection.try_into() else {
            return Ok(());
        };
        let Some(index) = self.expansion_index(guide) else {
            return Ok(());
        };
        let task = RustTyDecomposition::from_ty(ty, self.def_id);
        let Some(owner_projection) = LifetimeProjection::new(
            task,
            target.region(self.pcg_ctxt()),
            None,
            crate::encoders::ty::indirect::PrustiPcgCtxt,
        ) else {
            return Ok(());
        };
        let indirect = self
            .deps
            .require_dep::<IndirectPredicatesEnc>(owner_projection)?;
        let Some(array) = indirect.array else {
            return Ok(());
        };
        let mut old_outer = WandOldOuter::Label(label);
        let index = if fold {
            index
        } else {
            let at = match flow.long().base() {
                PcgLifetimeProjectionBase::Place(MaybeRemotePlace::Local(
                    MaybeLabelledPlace::Labelled(place),
                )) => Some(self.get_location_label(place.at())),
                _ => label.map(vir::OldLabel::Label),
            };
            self.vcx.maybe_apply_label(index, at)
        };
        let indexed = self.vcx.alloc(array.indexed.clone());
        let resources = if fold {
            self.encode_lifetime_projection_resources(
                target,
                &mut old_outer,
                None,
                Some((indexed, index, true)),
            )?
        } else {
            self.encode_lifetime_projection_resources(
                flow.long(),
                &mut old_outer,
                None,
                Some((indexed, index, true)),
            )?
        };
        let resources = self.vcx.mk_conj(&resources);
        let statements = if fold {
            array.indexed.operations(resources, index, true)
        } else {
            array.indexed.open_future(resources, index)
        }
        .map_err(|reason| self.unsupported_rvalue(reason.to_string(), self.current_span()))?;
        for statement in statements {
            self.stmt(statement);
        }
        Ok(())
    }

    pub(crate) fn repack_array_indirect(
        &mut self,
        expansion: &BorrowPcgExpansion<'vir>,
        fold: bool,
        label: Option<&'vir str>,
    ) -> EncodeResult<'vir, Option<(Vec<vir::Stmt<'vir>>, Vec<vir::ExprBool<'vir>>)>, E> {
        let PcgNode::LifetimeProjection(base) = expansion.base() else {
            return Ok(None);
        };
        let ty = base.base().place().ty(self.pcg_ctxt()).ty;
        if !matches!(ty.kind(), ty::Array(..) | ty::Slice(..)) {
            return Ok(None);
        }
        let task = RustTyDecomposition::from_ty(ty, self.def_id);
        let indirect = self
            .deps
            .require_dep::<IndirectPredicatesEnc>(base.with_base(task))?;
        let Some(array) = indirect.array else {
            return Ok(None);
        };
        let Some(index) = expansion
            .guide()
            .and_then(|guide| self.expansion_index(guide))
        else {
            return Err(self.unsupported_rvalue(
                "non-indexed expansion of collection referent resources".to_string(),
                self.current_span(),
            ));
        };
        let nodes = expansion.expansion();
        let [PcgNode::LifetimeProjection(element)] = nodes.as_slice() else {
            return Err(self.unsupported_rvalue(
                "non-singleton expansion of collection referent resources".to_string(),
                self.current_span(),
            ));
        };
        let capture_key = (
            base.base().place(),
            element.base().place(),
            element.region_idx().index(),
        );
        if fold && let Some(&(index, resources)) = self.array_resource_captures.get(&capture_key) {
            let statements =
                array
                    .indexed
                    .operations(resources, index, true)
                    .map_err(|reason| {
                        self.unsupported_rvalue(reason.to_string(), self.current_span())
                    })?;
            let frame = label
                .map(|label| array.indexed.exposed_footprint(resources, label))
                .into_iter()
                .collect();
            return Ok(Some((statements, frame)));
        }
        let (_, mut snap, ty, _) = self.encode_place_with_snap(element.base().place())?;
        let heap_label = match base.label().or(element.label()) {
            Some(LifetimeProjectionLabel::Location(at)) => Some(self.get_location_label(at)),
            _ => match element.base() {
                MaybeLabelledPlace::Labelled(place) => Some(self.get_location_label(place.at())),
                _ => match base.label() {
                    Some(LifetimeProjectionLabel::Location(at)) => {
                        Some(self.get_location_label(at))
                    }
                    _ => match base.base() {
                        MaybeLabelledPlace::Labelled(place) => {
                            Some(self.get_location_label(place.at()))
                        }
                        _ => label.map(vir::OldLabel::Label),
                    },
                },
            },
        };
        // An opening reads the newly exposed child in the current heap.
        let heap_label = if fold {
            heap_label
        } else {
            label.map(vir::OldLabel::Label)
        };
        // The index local may not yet exist at the owner's historical heap.
        // Freeze it at the child version before reading that older owner.
        let index_label = if fold {
            match element.label() {
                Some(LifetimeProjectionLabel::Location(at)) => Some(self.get_location_label(at)),
                _ => match element.base() {
                    MaybeLabelledPlace::Labelled(place) => {
                        Some(self.get_location_label(place.at()))
                    }
                    _ => heap_label,
                },
            }
        } else {
            heap_label
        };
        let index = self.vcx.maybe_apply_label(index, index_label);
        if fold || heap_label.is_some() {
            snap =
                self.indexed_element_snapshot(base.base().place(), element.base().place(), index)?;
        }
        let snap = self.vcx.maybe_apply_label(snap, heap_label);
        let (snap, index) = if !fold && label.is_none() {
            let saved_snap = self.new_tmp(snap.ty());
            let saved_index = self.new_tmp(index.ty());
            self.stmt(self.vcx.mk_pure_assign_stmt(saved_snap, snap));
            self.stmt(self.vcx.mk_pure_assign_stmt(saved_index, index));
            (saved_snap, saved_index)
        } else {
            (snap, index)
        };
        let task = RustTyDecomposition::from_ty(ty.ty, self.def_id);
        let child = self
            .deps
            .require_dep::<IndirectPredicatesEnc>(element.with_base(task))?;
        let indexed = self.vcx.alloc(array.indexed.clone());
        let resources = child
            .predicate_applications
            .iter()
            .map(|expr| {
                expr.reify(
                    self.vcx,
                    ExprInput {
                        heap_label,
                        indexed: Some((indexed, index, true)),
                        ..ExprInput::current(snap)
                    },
                )
            })
            .collect::<Vec<_>>();
        let mut resources = self.vcx.mk_conj(&resources);
        if !fold && label.is_none() {
            let (captured, statements) =
                crate::encoders::ty::indexed::IndexedResources::capture(resources, &mut |ty| {
                    self.new_tmp(ty)
                })
                .map_err(|reason| {
                    self.unsupported_rvalue(reason.to_string(), self.current_span())
                })?;
            for statement in statements {
                self.stmt(statement);
            }
            resources = captured;
            self.array_resource_captures
                .insert(capture_key, (index, resources));
        }
        let statements = array
            .indexed
            .operations(resources, index, fold)
            .map_err(|reason| self.unsupported_rvalue(reason.to_string(), self.current_span()))?;
        let frame = if fold {
            label
                .map(|label| array.indexed.exposed_footprint(resources, label))
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        Ok(Some((statements, frame)))
    }

    /// A future edge carries the updated element resources into its owner.
    /// Reversing it during restoration exposes those resources before the
    /// historical expansion reassembles the owner's original element.
    pub(crate) fn open_array_future(
        &mut self,
        flow: &BorrowFlowEdge<'vir>,
        label: Option<&'vir str>,
    ) -> EncodeResult<'vir, Vec<vir::Stmt<'vir>>, E> {
        // Outside a package the element's current resources are already
        // exposed. Only the wand antecedent brings a new indexed collection.
        if label.is_none() {
            return Ok(Vec::new());
        }
        let PcgLifetimeProjectionBase::Place(source) = flow.long().base() else {
            return Ok(Vec::new());
        };
        let Some(source) = source.as_local_place() else {
            return Ok(Vec::new());
        };
        let owner = flow.short().base().place();
        let element = source.place();
        let Some((parent, projection)) = element.last_projection() else {
            return Ok(Vec::new());
        };
        if parent != owner
            || !matches!(
                owner.ty(self.pcg_ctxt()).ty.kind(),
                ty::Array(..) | ty::Slice(..)
            )
        {
            return Ok(Vec::new());
        }
        let Ok(guide) = projection.try_into() else {
            return Ok(Vec::new());
        };
        let Some(index) = self.expansion_index(guide) else {
            return Ok(Vec::new());
        };
        let task = RustTyDecomposition::from_ty(owner.ty(self.pcg_ctxt()).ty, self.def_id);
        let indirect = self
            .deps
            .require_dep::<IndirectPredicatesEnc>(flow.short().with_base(task))?;
        let Some(array) = indirect.array else {
            return Ok(Vec::new());
        };
        let heap_label = match flow.long().label() {
            Some(LifetimeProjectionLabel::Location(at)) => Some(self.get_location_label(at)),
            _ => match source {
                MaybeLabelledPlace::Labelled(place) => Some(self.get_location_label(place.at())),
                _ => label.map(vir::OldLabel::Label),
            },
        };
        if let Some(package_label) = label {
            // Instantiate the finite collection's selector trigger in the
            // incoming wand state. Without this checked read, Silicon can
            // satisfy the ensuing unfold from the outer heap instead of the
            // quantified antecedent, consuming the collection we must return.
            let saved_index = self.vcx.maybe_apply_label(index, heap_label);
            let selected = self.indexed_element_snapshot(owner, element, saved_index)?;
            let selected = self.vcx.mk_local_labelled_old_expr(selected, package_label);
            let trigger = self.new_tmp(selected.ty());
            self.stmt(self.vcx.mk_pure_assign_stmt(trigger, selected));
        }
        let snap = self.indexed_element_snapshot(owner, element, index)?;
        let snap = self.vcx.maybe_apply_label(snap, heap_label);
        let index = self.vcx.maybe_apply_label(index, heap_label);
        let task = RustTyDecomposition::from_ty(element.ty(self.pcg_ctxt()).ty, self.def_id);
        let child = self
            .deps
            .require_dep::<IndirectPredicatesEnc>(flow.long().with_base(task))?;
        let indexed = self.vcx.alloc(array.indexed);
        let resources = child
            .predicate_applications
            .iter()
            .map(|expr| {
                expr.reify(
                    self.vcx,
                    ExprInput {
                        heap_label,
                        indexed: Some((indexed, index, true)),
                        ..ExprInput::current(snap)
                    },
                )
            })
            .collect::<Vec<_>>();
        indexed
            .open_future(self.vcx.mk_conj(&resources), index)
            .map_err(|reason| self.unsupported_rvalue(reason.to_string(), self.current_span()))
    }

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
            let extent = if matches!(ty.ty.kind(), ty::Slice(..)) {
                self.resource_extent(place_res.expr.metadata())?
            } else {
                None
            };
            for proj in projs {
                let indirect = self
                    .deps
                    .require_dep::<IndirectPredicatesEnc>(proj.with_base(task))
                    .unwrap();
                inv.extend(indirect.predicate_applications.iter().map(|p| {
                    p.reify(
                        self.vcx,
                        ExprInput {
                            extent,
                            ..ExprInput::current(
                                ty_out.ref_to_snap(place_res.expr.expect_predicate()),
                            )
                        },
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
            .filter_map(|edge| {
                let coupled = match edge.kind() {
                    BorrowPcgEdgeKind::Coupled(edge) => edge.clone(),
                    BorrowPcgEdgeKind::Abstraction(edge @ AbstractionEdge::Loop(_)) => {
                        edge.clone().into_singleton_coupled_edge()
                    }
                    _ => return None,
                };
                Some((coupled, self.edge_condition(&edge.to_owned_edge())))
            })
            .collect();

        // A restoration target such as `*values` needs the shallow reference
        // value to identify its owner. Preserve that value's predicate without
        // also claiming the referent, which remains owned through the iterator.
        let mut reference_frames = FxHashSet::default();
        for (edge, _) in &edges {
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
        // A reference's shallow value identifies its referent after loop
        // havoc, including owners whose resources are held by a call wand.
        // Writes through the reference do not change that value. Assigning,
        // moving, or mutably borrowing the reference slot does, so exclude it.
        for (local, decl) in self.body.local_decls.iter_enumerated() {
            let place: Place<'vir> = local.into();
            if matches!(decl.ty.kind(), ty::Ref(..))
                && state
                    .capabilities()
                    .capability(place, self.pcg_ctxt())
                    .is_some_and(|cap| !cap.is_write())
                && !loop_place_usages.iter().any(|(used, usage)| {
                    used.local == local
                        && used.projection.is_empty()
                        && !matches!(usage, pcg::r#loop::PlaceUsageType::Read)
                })
            {
                reference_frames.insert(place);
            }
        }
        let mut snapshots = Vec::new();
        // Keep the snapshot stored in an unchanged ghost local. Its predicate
        // alone would otherwise allow the verifier to forget the value at the
        // loop boundary. This does not add resources behind that value.
        for (place, capability) in &loop_invariant_place_capabilities {
            if capability.is_write() || !place.projection.is_empty() {
                continue;
            }
            let ty::Adt(adt, _) = place.ty(ctxt).ty.kind() else {
                continue;
            };
            if !adt.is_struct()
                || !prusti_interface::environment::EnvQuery::new(self.vcx.tcx())
                    .is_adt_in_crate(*adt, "prusti_contracts")
                || adt.non_enum_variant().name.as_str() != "Ghost"
                || loop_place_usages.iter().any(|(used, usage)| {
                    (place.is_prefix_of(**used) || used.is_prefix_of(**place))
                        && !matches!(usage, pcg::r#loop::PlaceUsageType::Read)
                })
            {
                continue;
            }
            let (_, snap, _, _) = self.encode_place_with_snap(*place)?;
            let name = vir::vir_format!(self.vcx, "_loop_ghost{}", self.tmp_ctr);
            self.tmp_ctr += 1;
            let decl = self.vcx.mk_local_decl(name, snap.ty());
            self.stmt(self.vcx.mk_local_decl_stmt(decl, None));
            let saved = self.vcx.mk_local_ex(decl);
            inv.push(self.vcx.mk_eq_expr(snap, saved));
            snapshots.push((saved, snap));
        }
        for place in reference_frames {
            let (encoded, snap, _, _) = self.encode_place_with_snap(place)?;
            let task = RustTyDecomposition::from_ty(place.ty(ctxt).ty, self.def_id);
            let ty = self.deps.require_dep::<TyUseImpureEnc>(task)?;
            if !loop_invariant_place_capabilities
                .get(&place)
                .is_some_and(|cap| !cap.is_write())
            {
                inv.push(ty.ref_to_pred(self.vcx, encoded.expr.expect_predicate(), None));
            }
            let name = vir::vir_format!(self.vcx, "_loop_ref{}", self.tmp_ctr);
            self.tmp_ctr += 1;
            let decl = self.vcx.mk_local_decl(name, snap.ty());
            self.stmt(self.vcx.mk_local_decl_stmt(decl, None));
            let saved = self.vcx.mk_local_ex(decl);
            inv.push(self.vcx.mk_eq_expr(snap, saved));
            snapshots.push((saved, snap));
        }
        if !snapshots.is_empty() {
            self.loop_snapshot_frames.push(LoopSnapshotFrame {
                loop_id,
                head: self.current_block_label.unwrap(),
                snapshots,
            });
        }
        // A function-call wand can contain expiry pledges in addition to its
        // resource predicates. Rebuilding it as a resource-only loop wand
        // would request a different wand and lose the value relation.
        let mut retained_wands = FxHashMap::default();
        for (edge, condition) in edges {
            if let PcgCoupledEdgeKind(FunctionCallOrLoop::FunctionCall(call)) = &edge
                && !self
                    .fpcs_analysis
                    .analysis()
                    .loop_analysis()
                    .in_loop(call.metadata().location().block, loop_id)
            {
                for (id, wand) in self.encode_function_call_wands(&edge)? {
                    let (_, guards) = retained_wands
                        .entry((call.metadata().location().block, id))
                        .or_insert_with(|| (wand, Vec::new()));
                    guards.push(self.vcx.mk_conj(&condition));
                }
                continue;
            }
            let mut let_bind = WandOldOuter::LetBind(Vec::new());
            let (wand, _) = self.coupled_wand(&edge, &mut let_bind)?;
            let wand = self.with_restoration_wand(wand, &edge, &mut let_bind, false, false)?;
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

        // Retain each obligation only where its call ran. Several PCG edges
        // may contribute guards for the same obligation; use their disjunction.
        for (wand, guards) in retained_wands.into_values() {
            inv.push(
                self.vcx
                    .mk_bin_op_expr(
                        vir::BinOpKind::Implies,
                        self.vcx.mk_disj(&guards),
                        self.vcx.mk_wand_expr(wand),
                    )
                    .downcast_ty(),
            );
        }

        Ok((inv, loop_invariant_place_capabilities.into_iter().collect()))
    }

    /// Initialise shallow reference and ghost snapshots on entry edges only.
    /// Capturing them on a backedge would erase the equality that the invariant
    /// must preserve. Pre-loop copies belong to the entry path even when their
    /// MIR block is syntactically inside the loop.
    pub(crate) fn capture_loop_snapshot_frames(&mut self) {
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
                    .loop_snapshot_frames
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
        self.encode_lifetime_projection_with_snapshot(r, old_outer, None)
    }

    pub(crate) fn encode_lifetime_projection_with_snapshot<
        T: PcgLifetimeProjectionBaseLike<'vir>,
    >(
        &mut self,
        r: LifetimeProjection<'vir, T>,
        old_outer: &mut WandOldOuter<'vir>,
        saved: Option<vir::ExprSnap<'vir>>,
    ) -> EncodeResult<'vir, Vec<vir::ExprBool<'vir>>, E> {
        self.encode_lifetime_projection_resources(r, old_outer, saved, None)
    }

    fn encode_lifetime_projection_resources<T: PcgLifetimeProjectionBaseLike<'vir>>(
        &mut self,
        r: LifetimeProjection<'vir, T>,
        old_outer: &mut WandOldOuter<'vir>,
        saved: Option<vir::ExprSnap<'vir>>,
        indexed: Option<(
            &'vir crate::encoders::ty::indexed::IndexedResources<'vir>,
            vir::ExprInt<'vir>,
            bool,
        )>,
    ) -> EncodeResult<'vir, Vec<vir::ExprBool<'vir>>, E> {
        let place = r.base().to_pcg_lifetime_projection_base();
        let heap_label = match r.label() {
            Some(LifetimeProjectionLabel::Location(at)) => Some(self.get_location_label(at)),
            _ => None,
        };
        let (place_snap, ty) = if let Some(saved) = saved {
            let PcgLifetimeProjectionBase::Place(place) = place else {
                unreachable!()
            };
            let p = Self::get_place(place);
            (saved, (*p).ty(self.local_decls, self.vcx.tcx()))
        } else {
            match place {
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
                    // Moving the base later must not replace the earlier value
                    // selected by its lifetime projection (e.g. an iterator's
                    // tail before `next`). When the base was moved first, retain
                    // its saved shallow value and use the projection's heap label
                    // only for resources reached through that value.
                    let (_, snap, ty, _) = self.encode_place_with_snap(p.place())?;
                    (self.vcx.maybe_apply_label(snap, heap_label), ty)
                }
                PcgLifetimeProjectionBase::Place(p) => {
                    let (snap, ty, _) = self.encode_maybe_remote_place_snap(p, old_outer)?;
                    (snap, ty)
                }
                PcgLifetimeProjectionBase::Const(c) => todo!("{c:?}"),
            }
        };
        let ty = RustTyDecomposition::from_ty(ty.ty, self.def_id);
        let extent = if matches!(ty.ty.specifics, super::super::ty::TySpecifics::ArrayLike(ref array) if array.slice)
        {
            let PcgLifetimeProjectionBase::Place(place) = place else {
                unreachable!()
            };
            let metadata = self.encode_place(Self::get_place(place))?.expr.metadata();
            let metadata = metadata.map(|metadata| self.configure_old(place, metadata, old_outer));
            let metadata =
                metadata.map(|metadata| self.vcx.maybe_apply_label(metadata, heap_label));
            self.resource_extent(metadata)?
        } else {
            None
        };
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
                        extent,
                        indexed,
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

    pub(crate) fn mk_wand_outer<T: SnapOrRef>(
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

pub(crate) trait SnapOrRef: vir::CompType {
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
