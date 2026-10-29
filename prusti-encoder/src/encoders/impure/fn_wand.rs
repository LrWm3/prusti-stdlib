use crate::encoders::{
    EncodeResult, ImpureEncVisitor, MirLocalDefEncOutput, MirSpecEnc, Pure,
    mir_fn::RustSignature,
    mir_pure::PostCapture,
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
use vir::{CastType, HasType};

/// Encodes the magic wands given a function signature.
pub struct WandEnc;

#[derive(Clone, Debug)]
pub enum WandEncError {
    Unsupported(String),
}

impl<'vir, E: TaskEncoder> ImpureEncVisitor<'vir, '_, E> {
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

        for wand_data in self.wands.viper_wands() {
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
                        .filter(|a| !opaque_action(a.edge()))
                        .map(|a| a.edge().clone()),
                );
            }
            let Some(wand) = self.wands.mk_wand(
                &wand_data,
                args,
                None,
                Some(label),
                None,
                None,
                &[],
                self.vcx,
                self.deps,
            ) else {
                continue;
            };
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
            let mut package_script = self.pcs_package_script_filtered(
                final_borrow_state,
                rhs,
                label,
                &prior_edges,
                |edge| restores_opaque || !opaque_action(edge),
            )?;

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
                visitor,
            )?
            .into_iter()
            .map(|(_, wand, _, _)| wand)
            .collect())
    }

    /// The index identifies an obligation in this function's retained wand
    /// list, before substitution or omission of empty instantiated wands.
    pub(crate) fn selected_call_wands<E: TaskEncoder>(
        &self,
        arguments: &[vir::ExprSnap<'vir>],
        label_pre: &'vir str,
        label_post: &'vir str,
        call_ctx: GArgs<'vir>,
        extra_pledges: &[PledgeExpr<'vir>],
        selected: Option<&[usize]>,
        visitor: &mut ImpureEncVisitor<'vir, '_, E>,
    ) -> EncodeResult<
        'vir,
        Vec<(
            usize,
            vir::Wand<'vir>,
            Vec<vir::Stmt<'vir>>,
            Vec<vir::Stmt<'vir>>,
        )>,
        E,
    > {
        let result = visitor
            .vcx
            .mk_local_labelled_old_expr(arguments[mir::RETURN_PLACE.as_usize()], label_post);
        let args = (1..arguments.len()).map(|l| {
            visitor
                .vcx
                .mk_local_labelled_old_expr(arguments[l], label_pre)
        });
        let args = PledgeExpr::pledge_args(result, args);
        let mut wands = Vec::new();
        for (id, wand_data) in self.wands.iter().enumerate() {
            if selected.is_some_and(|ids| !ids.contains(&id)) {
                continue;
            }
            let Some(wand) = self.mk_wand(
                wand_data,
                args,
                Some(label_pre),
                Some(label_post),
                None,
                Some(call_ctx),
                extra_pledges,
                visitor.vcx,
                visitor.deps,
            ) else {
                continue;
            };
            let signature = RustSignature::new(self.function_data.def_id());
            let mut convert = |node: FunctionShapeNode<Generalized>,
                               pack|
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
                    Ok(conversion.at_snapshot_on_heap(
                        args[node.mir_local()],
                        pack,
                        if pack { label_post } else { label_pre },
                    ))
                } else {
                    Ok(Vec::new())
                }
            };
            let mut before = Vec::new();
            for node in &wand_data.lhs {
                before.extend(convert(*node, true)?);
            }
            let mut after = Vec::new();
            for node in &wand_data.rhs {
                after.extend(convert((*node).into(), false)?);
            }
            wands.push((id, wand, before, after));
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
            let targets = wand
                .flow_lhs
                .as_ref()
                .unwrap_or(&wand.lhs)
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
            if !sources.iter().all(|node| inputs.contains(node))
                || !targets.iter().all(|node| outputs.contains(node))
            {
                return Err(
                    "function-call edge covers only part of a restoration obligation".to_string(),
                );
            }
            selected.push(id);
        }
        Ok(selected)
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
        debug_assert!(!wand_data.lhs.is_empty());
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
                    None,
                    |i| pledge_args[i],
                )
            })
            .collect::<Vec<_>>();
        let frame_values = frames
            .iter()
            .map(|frame| physical_frame(vcx, *frame))
            .collect::<Vec<_>>();
        let rhs = wand_data.rhs.iter().filter_map(|g| {
            self.encode_predicates_for_function_shape_node(
                vcx,
                deps,
                *g,
                call_ctx,
                Some(pledge_old_label.map_or(vir::OldLabel::None, vir::OldLabel::Label)),
                None,
                |i| pledge_args[i],
            )
        });
        let rhs = rhs
            .chain(frames.iter().copied())
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

/// A physical frame must preserve its stored snapshot as well as permission.
/// Indirect mutable-reference leaves are represented by `p_Param`.
fn physical_frame<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    resources: vir::ExprBool<'vir>,
) -> vir::ExprBool<'vir> {
    match resources.kind {
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
    /// Lifetime projections on the left-hand side of the wand. Guaranteed to be
    /// non-empty.
    lhs: Vec<WandLhsKey>,
    pledges: EncodedPledges<'vir>,
    /// Permissions restored by an earlier wand, consumed and returned unchanged.
    frames: Vec<WandRhsKey>,
    /// Original shape targets retained for checking PCG call-edge coverage.
    flow_lhs: Option<Vec<WandLhsKey>>,
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
                .filter_map(|hyper_edge| {
                    let (sources, mut targets) = hyper_edge.into_tuple();
                    // We don't want to emit an identity wand, like P --* P. This can happen when
                    // PCG returns self-edges, like for fn(x: &'a mut &'b i32) where 'b is in
                    // invariant position and we therefore have an edge x|'b -> x|'b.
                    // Currently, these edges also prevent us from emitting indirect postconditions.
                    // TODO: we might want to emit these identity wands in the future to attach functional
                    // specifications to them. We still need to emit the resources on the wand's LHS.
                    let mut sources_as_nodes = sources
                        .iter()
                        .map(|&s| s.to_function_shape_node())
                        .collect::<Vec<_>>();
                    sources_as_nodes.sort();
                    targets.sort();
                    if sources_as_nodes == targets {
                        return None;
                    }
                    Some(WandData::new(targets, sources, pledges.clone()))
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
            !edges.iter().any(|(source, target)| {
                source.mir_local() == node.mir_local()
                    && source.region_idx().index() == 0
                    && target.mir_local() == mir::RETURN_PLACE
            })
        };
        let wands = wands
            .into_iter()
            .map(|mut wand| {
                wand.lhs.retain(|node| visible(*node));
                wand
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
                if targets.is_empty() {
                    return Err(WandEncError::Unsupported(
                        "empty dependent resource restoration".into(),
                    ));
                }
                previous.push((original.clone(), sources.clone()));
                let mut wand = WandData::new(targets, sources, Vec::new());
                wand.frames = frames;
                wand.flow_lhs = Some(original);
                stratified.push(wand);
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
                if resource_count > 1 {
                    return Err(WandEncError::Unsupported(
                        "pledges on a function whose result contains borrows that expire \
                         separately require an explicit input expiry selector"
                            .into(),
                    ));
                }
                if wands.is_empty() {
                    return Err(WandEncError::Unsupported(
                        "expiry pledge has no borrowed input to restore".into(),
                    ));
                }
                0
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
