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
    },
};
use pcg::{
    borrow_pcg::{
        FunctionData, FunctionShape, FunctionShapeInput, FunctionShapeNode, FunctionShapeOutput,
        GeneralizedLifetime, MakeFunctionShapeError,
        borrow_pcg_edge::{BlockedNode, BorrowPcgEdge},
        region_projection::{ExtractRegionsCtxt, Generalized, LifetimeProjection, PcgRegion},
        state::BorrowsState,
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
                    let local = g.mir_local();
                    let node: FunctionShapeNode<Generalized> = (*g).into();
                    let generalized: IndexVec<_, GeneralizedLifetime<'vir>> = self
                        .vcx
                        .tcx()
                        .extract_regions(node.ty(self.wands.fn_sig(self.vcx, None)));
                    let actual: IndexVec<_, PcgRegion<'vir>> = self
                        .pcg_ctxt()
                        .extract_regions(self.body.local_decls[local].ty);
                    // Explicit signature regions and MIR regions have the
                    // same traversal order. Opaque types can hide regions,
                    // so retain the previous place-based lookup when the
                    // two lists do not have a direct correspondence.
                    let root: BlockedNode<'vir> = if generalized.len() == actual.len()
                        && generalized
                            .iter()
                            .all(|r| matches!(r, GeneralizedLifetime::Region(_)))
                    {
                        LifetimeProjection::from_index(
                            RemotePlace::new(local),
                            g.region_idx().index().into(),
                        )
                        .into()
                    } else {
                        mir::Place::from(local).into()
                    };
                    (root, resources.into_iter().collect())
                })
                .collect();
            let mut package_script =
                self.pcs_package_script(final_borrow_state, rhs, label, already_unblocked)?;

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
    fn predicates_for_function_shape_node(
        &self,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, impl TaskEncoder>,
        g: FunctionShapeNode<Generalized>,
        call_ctx: WandCallContext<'vir>,
    ) -> Option<Vec<vir::ExprGenBool<'vir, ExprInput<'vir>, vir::ExprKind<'vir>>>> {
        let arg_ty = g.ty(self.fn_sig(vcx, call_ctx));
        let decomp = RustTyDecomposition::from_ty(arg_ty, self.g_params(vcx, call_ctx));
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
        let local_snap = snap(local);
        Some(
            vcx.mk_conj(
                &predicates
                    .iter()
                    .map(|p| {
                        p.reify(
                            vcx,
                            ExprInput {
                                snap: local_snap,
                                heap_label,
                                bindings,
                                guard: None,
                            },
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
        )
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
    ) -> Vec<vir::Wand<'vir>> {
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
        for wand_data in self.viper_wands() {
            let Some(wand) = self.mk_wand(
                &wand_data,
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
            wands.push(wand);
        }
        wands
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
        });
        let pledge_expr = |pledge: &PledgeExpr<'vir>| match pledge_old_label {
            Some(label) => {
                vcx.with_local_subst(generics, || pledge.expr_at_label(spec_args, label, capture))
            }
            None => pledge.expr(spec_args, capture),
        };
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
}

impl<'vir> WandData<'vir> {
    pub fn new(lhs: Vec<WandLhsKey>, rhs: Vec<WandRhsKey>, pledges: EncodedPledges<'vir>) -> Self {
        debug_assert!(!lhs.is_empty());
        debug_assert!(!rhs.is_empty());
        Self { rhs, lhs, pledges }
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
                assert!(spec.pledges.is_empty());
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
                .select_wands(wands, !pledges.is_empty(), &edges, vcx, deps)
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
        has_pledges: bool,
        edges: &FxHashSet<(WandRhsKey, WandLhsKey)>,
        vcx: &'vir vir::VirCtxt<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, E>,
    ) -> Result<Vec<WandData<'vir>>, WandEncError> {
        let mut has_resources = |g: FunctionShapeNode<Generalized>| {
            self.predicates_for_function_shape_node(vcx, deps, g, None)
                .is_some()
        };
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
        let (mut wands, resourceless): (Vec<_>, Vec<_>) = wands
            .into_iter()
            .partition(|wand_data| wand_data.rhs.iter().any(|g| has_resources((*g).into())));
        // A wand gives back its sources only once all of its targets have
        // expired. That matches the signature only if each source with
        // resources flows into each target with resources.
        for wand_data in &wands {
            let sources = wand_data
                .rhs
                .iter()
                .filter(|g| has_resources((**g).into()))
                .collect::<Vec<_>>();
            let targets = wand_data
                .lhs
                .iter()
                .filter(|g| has_resources(**g))
                .collect::<Vec<_>>();
            let precise = sources.iter().all(|source| {
                let mut targets = targets.iter();
                targets.all(|target| edges.contains(&(**source, **target)))
            });
            if !precise {
                return Err(WandEncError::Unsupported(
                    "borrows in the result that expire separately but depend on a common \
                     argument are not supported"
                        .to_string(),
                ));
            }
        }
        if has_pledges {
            if wands.is_empty() {
                wands.extend(resourceless.into_iter().take(1));
            } else if wands.len() > 1 {
                // It is unclear which expiry the pledges refer to.
                return Err(WandEncError::Unsupported(
                    "pledges on a function whose result contains borrows that expire \
                     separately are not supported"
                        .to_string(),
                ));
            }
        }
        Ok(wands)
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
