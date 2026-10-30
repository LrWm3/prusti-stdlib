//! Checked symbolic sequence conversions for finite element-resource shapes.
use super::{
    RustTyDecomposition, RustTyNormalized,
    generics::{GArgs, GArgsCastEnc, GParams, GenericParams, GenericParamsEnc},
    indexed::quantify_resources,
    resource_conversion::ResourceConversionEnc,
    resources::{ResourceUse, fields_sequence_state, state_type, unit_metadata},
};
use std::hash::{Hash, Hasher};
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::{CastType, MethodIdn};

type Arguments = (vir::Seq, vir::Int, vir::ManyTyVal, vir::ManyCSnap);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ResourceCollectionTask<'vir> {
    element: RustTyDecomposition<'vir>,
    aggregate: bool,
}

impl<'vir> ResourceCollectionTask<'vir> {
    pub fn pointwise(element: RustTyDecomposition<'vir>) -> Self {
        Self {
            element,
            aggregate: false,
        }
    }

    pub fn aggregate(element: RustTyDecomposition<'vir>) -> Self {
        Self {
            element,
            aggregate: true,
        }
    }
}

pub(crate) struct ResourceCollectionEnc;
#[derive(Clone)]
pub(crate) struct ResourceCollection<'vir> {
    pack: MethodIdn<'vir, Arguments>,
    unpack: MethodIdn<'vir, Arguments>,
    params: GenericParams<'vir>,
    raw_sequence: Option<ResourceSequenceState<'vir>>,
    element_state: ResourceElementState<'vir>,
}
impl<'vir> ResourceCollection<'vir> {
    pub fn observe<Curr, Next>(
        &self,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        fields_sequence_state(self.observe_elements(sequence))
    }

    pub fn observe_elements<Curr, Next>(
        &self,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
    ) -> vir::ExprGenSeq<'vir, Curr, Next> {
        self.raw_sequence
            .as_ref()
            .expect("aggregate resource collection observer")
            .observe_elements(sequence)
    }

    pub fn observe_element<Curr, Next>(
        &self,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
        index: vir::ExprGenInt<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        self.element_state.observe(sequence, index)
    }

    pub fn statements(&self, sequence: vir::ExprSeq<'vir>, pack: bool) -> Vec<vir::Stmt<'vir>> {
        vir::with_vcx(|vcx| {
            let zero = vcx.mk_uint::<0>();
            vec![
                (if pack { self.pack } else { self.unpack })(
                    sequence,
                    zero,
                    self.params.ty_exprs(),
                    self.params.const_exprs(),
                )
                .alloc(),
            ]
        })
    }
}
impl ResourceCollectionEnc {
    fn normalize_element<'vir>(task: &RustTyDecomposition<'vir>) -> RustTyDecomposition<'vir> {
        vir::with_vcx(|vcx| {
            // A concrete element's resource representation does not depend on
            // its source lifetime or unrelated enclosing const/type parameters.
            // Share its observer between contracts and body conversions.
            let args = vcx
                .tcx()
                .erase_and_anonymize_regions(vcx.tcx().mk_args(task.args.args()));
            let context = GParams::empty();
            let ground = args.iter().flat_map(|arg| arg.walk()).all(|arg| {
                context.check_arg(arg)
                    && !arg.as_type().is_some_and(|ty| {
                        matches!(
                            ty.kind(),
                            prusti_rustc_interface::middle::ty::TyKind::Alias(..)
                        )
                    })
            });
            if ground {
                RustTyDecomposition {
                    ty: task.ty,
                    args: GArgs::new(context, args),
                }
            } else {
                *task
            }
        })
    }
}

impl TaskEncoder for ResourceCollectionEnc {
    task_encoder::encoder_cache!(ResourceCollectionEnc);
    const ENCODER_NAME: &'static str = "indexed resource conversion encoder";
    type TaskDescription<'vir> = ResourceCollectionTask<'vir>;
    type OutputFullDependency<'vir> = ResourceCollection<'vir>;
    type OutputFullLocal<'vir> = Vec<vir::Method<'vir>>;
    type EncodingError = String;
    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        ResourceCollectionTask {
            element: Self::normalize_element(&task.element),
            aggregate: task.aggregate,
        }
    }
    fn describe_error(error: String) -> String {
        error
    }
    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let aggregate = task.aggregate;
        let task = &task.element;
        deps.require_ref::<super::resources::ResourceSequenceEnc>(())?;
        let conversion = deps.require_dep::<ResourceConversionEnc>(*task)?;
        if !conversion.supports_indexed_resources() {
            return Err(task_encoder::EncodeFullError::EncodingError(
                "nested collection resource conversion is unsupported".into(),
                None,
            ));
        }
        let element_state = deps.require_dep::<ResourceElementStateEnc>(*task)?;
        let raw_observer = if aggregate {
            Some(deps.require_dep::<ResourceSequenceStateEnc>(*task)?)
        } else {
            None
        };
        let caster = deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
            (!task.ty.specifics.is_param()).then_some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: *task,
            }),
        )?;
        let params = deps.require_dep::<GenericParamsEnc>(task.args.context())?;
        let resource = ResourceUse::new(deps, *task)?;
        let unit = unit_metadata(deps)?;
        vir::with_vcx(|vcx| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            // rustc's interned types hash by address. Use their description so
            // identical inputs keep the same helper names between runs.
            format!("{task:?}").hash(&mut hash);
            let suffix = hash.finish();
            let seq_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
            let sequence = vcx.mk_local_decl("conversion_elements", seq_ty);
            let start = vcx.mk_local_decl("conversion_start", vir::TYPE_INT);
            let seq = vcx.mk_local_ex(sequence);
            let i = vcx.mk_local_ex(start);
            let length = vcx.mk_collection_len_expr(seq);
            let zero = vcx.mk_uint::<0>();
            let next = (vir::expr! { (i) + ([vcx.mk_uint::<1>()]) }).downcast_ty::<vir::Int>();
            let arity = (seq_ty, vir::TYPE_INT, params.ty_args(), params.const_args());
            let frame_decl =
                vcx.mk_local_decl("conversion_initial_state", vcx.mk_ty_seq(state_type()));
            let frame = vcx.mk_local_ex(frame_decl);
            let pack = MethodIdn::<Arguments>::new(
                vir::vir_format_identifier!(
                    vcx,
                    "resource_pack_{}{suffix:x}",
                    if aggregate { "aggregate_" } else { "" }
                ),
                arity,
            );
            let unpack = MethodIdn::<Arguments>::new(
                vir::vir_format_identifier!(
                    vcx,
                    "resource_unpack_{}{suffix:x}",
                    if aggregate { "aggregate_" } else { "" }
                ),
                arity,
            );
            let raw_sequence = vir::FunctionIdn::<Arguments, vir::Seq>::new(
                vir::vir_format_identifier!(vcx, "resource_raw_sequence_{suffix:x}"),
                arity,
                vcx.mk_ty_seq(state_type()),
            );
            let sequence_state = |group: bool, index| {
                if group {
                    resource.sequence_state(seq, index)
                } else {
                    raw_sequence.call()(seq, index, params.ty_exprs(), params.const_exprs())
                }
            };
            let state = |group: bool, index| {
                if group {
                    if aggregate {
                        element_state.observe_group(seq, index)
                    } else {
                        resource.sequence_element_state(seq, index)
                    }
                } else {
                    element_state.observe(seq, index)
                }
            };
            let element = |index| {
                caster.cast_to_caller_ctx(
                    vcx.mk_seq_index_expr(seq, index).downcast_ty::<vir::Snap>(),
                )
            };
            let raw_element = |index| {
                vcx.mk_seq_index_expr(seq, index)
                    .downcast_ty::<vir::PSnap>()
            };
            let permission = |group: bool, index| {
                if group {
                    conversion.grouped_generic(raw_element(index), index)
                } else {
                    conversion.indexed_resources(element(index), index)
                }
            };
            let binder = vcx.mk_local_decl("conversion_index", vir::TYPE_INT);
            let j = vcx.mk_local_ex(binder);
            let variables = vcx.alloc_slice(&[binder.as_dyn()]);
            let range = |lo: vir::ExprInt<'vir>| vir::expr! { ((lo) <= (j)) && ((j) < (length)) };
            let resources = |group, lo| {
                quantify_resources(
                    vcx,
                    variables,
                    // Observe the same lookup when checking an element and
                    // when instantiating its quantified permission. Inferred
                    // triggers can retain a concrete snapshot projection that
                    // disappears after constructor normalization.
                    if aggregate {
                        &[]
                    } else {
                        vcx.alloc_slice(&[vcx.mk_trigger(&[raw_element(j).as_dyn()])])
                    },
                    range(lo),
                    permission(group, j),
                )
            };
            // Array snapshots store generic elements; retain their concrete representation
            // identity explicitly when the element has been instantiated.
            let normalization = |index| {
                vcx.mk_eq_expr(
                    caster.cast_to_callee_ctx(element(index)),
                    vcx.mk_seq_index_expr(seq, index).downcast_ty::<vir::Snap>(),
                )
            };
            let typed = |lo| {
                vcx.mk_forall_expr(
                    variables,
                    // The canonical lookup is present whenever either side
                    // observes an element. Inferred constructor/cast triggers
                    // may disappear while the solver normalizes snapshots,
                    // hiding the recursively established suffix typing fact.
                    if aggregate {
                        &[]
                    } else {
                        vcx.alloc_slice(&[vcx.mk_trigger(&[raw_element(j).as_dyn()])])
                    },
                    vir::expr! { ([range(lo)]) ==> ([normalization(j)]) },
                )
            };
            let mut methods = Vec::new();
            // Unfold these recursive observers only in the conversion proof.
            // Client frames use their checked contracts, preventing unrelated
            // heap observations from repeatedly instantiating recursive tails.
            let revealed_sequence_state =
                |group, index| vcx.reveal_function(sequence_state(group, index));
            for packing in [true, false] {
                let source = !packing;
                let target = packing;
                let saved_decl = vcx.mk_local_decl("conversion_saved_state", state_type());
                let saved = vcx.mk_local_ex(saved_decl);
                if !aggregate {
                    // A lifetime-resource boundary needs elementwise preservation.
                    // Comparing aggregate observers additionally forces Silicon to
                    // equate whole quantified heap maps on each recursive call.
                    // Keep that stronger contract for whole-value array conversions.
                    let mut body = vec![vcx.mk_local_decl_stmt(saved_decl, Some(state(source, i)))];
                    if !packing {
                        let vir::ExprKindData::PredicateApp(wrapper) = permission(true, i).kind
                        else {
                            unreachable!()
                        };
                        let vir::ExprKindData::PredicateApp(group) =
                            resource.apply_generic(raw_element(i), unit).kind
                        else {
                            unreachable!()
                        };
                        body.push(vcx.mk_assert_stmt(vcx.mk_unfolding_expr(
                            wrapper,
                            vcx.mk_unfolding_expr(group, normalization(i)),
                        )));
                    }
                    body.extend(conversion.indexed_statements(element(i), i, packing));
                    body.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(state(target, i), saved)));
                    body.push(
                        (if packing { pack } else { unpack })(
                            seq,
                            next,
                            params.ty_exprs(),
                            params.const_exprs(),
                        )
                        .alloc(),
                    );
                    body.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(state(target, i), saved)));
                    let statements = [vcx.mk_if_stmt(
                        vir::expr! { (i) < (length) },
                        vcx.alloc_slice(&body),
                        &[],
                    )];
                    let block = vcx.mk_cfg_block(
                        &vir::CfgBlockLabelData::Start,
                        &[],
                        vcx.alloc_slice(&statements),
                        &vir::TerminatorStmtGenData::Exit,
                    );
                    let access = super::indexed::observe_element(raw_element(j), j);
                    let posts = [
                        typed(i),
                        resources(target, i),
                        vcx.mk_forall_expr(
                            variables,
                            vcx.alloc_slice(&[
                                // A loop wand can connect several conversion
                                // heaps before observing the restored state.
                                // Ground each frame from the pure element key,
                                // as well as from a heap-specific observation.
                                vcx.mk_trigger(&[raw_element(j).as_dyn()]),
                                vcx.mk_trigger(&[access.as_dyn()]),
                                vcx.mk_trigger(&[state(target, j).as_dyn()]),
                            ]),
                            vir::expr! { ([range(i)]) ==> (([state(target, j)]) ==
                            ([vcx.mk_old_expr(state(source, j))])) },
                        ),
                    ];
                    let mut pres = vec![
                        vir::expr! { ((zero) <= (i)) && ((i) <= (length)) },
                        resources(source, i),
                    ];
                    if packing {
                        pres.push(typed(i));
                    }
                    methods.push(vcx.mk_method(
                        if packing { pack } else { unpack },
                        (sequence, start, params.ty_decls(), params.const_decls()),
                        &[],
                        vcx.alloc_slice(&pres),
                        vcx.alloc_slice(&posts),
                        Some(vcx.alloc_slice(&[block])),
                    ));
                    continue;
                }
                let tail_decl =
                    vcx.mk_local_decl("conversion_saved_tail", vcx.mk_ty_seq(state_type()));
                let tail = vcx.mk_local_ex(tail_decl);
                let mut body = vec![
                    vcx.mk_local_decl_stmt(saved_decl, Some(state(source, i))),
                    vcx.mk_local_decl_stmt(tail_decl, Some(revealed_sequence_state(source, next))),
                ];
                if !packing {
                    // The group's body establishes the element's type. Open it
                    // only for this checked observation before converting the
                    // canonical generic snapshot to the concrete representation.
                    let vir::ExprKindData::PredicateApp(wrapper) = permission(true, i).kind else {
                        unreachable!()
                    };
                    let vir::ExprKindData::PredicateApp(group) =
                        resource.apply_generic(raw_element(i), unit).kind
                    else {
                        unreachable!()
                    };
                    body.push(vcx.mk_assert_stmt(vcx.mk_unfolding_expr(
                        wrapper,
                        vcx.mk_unfolding_expr(group, normalization(i)),
                    )));
                }
                let assembled = vcx.mk_seq_concat_expr(
                    vcx.mk_seq_literal_expr(vcx.alloc_slice(&[saved]), state_type()),
                    tail,
                );
                body.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(frame, assembled)));
                body.extend(conversion.indexed_statements(element(i), i, packing));
                body.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(state(target, i), saved)));
                body.push(
                    vcx.mk_assert_stmt(vcx.mk_eq_expr(revealed_sequence_state(source, next), tail)),
                );
                body.push(
                    (if packing { pack } else { unpack })(
                        seq,
                        next,
                        params.ty_exprs(),
                        params.const_exprs(),
                    )
                    .alloc(),
                );
                body.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(state(target, i), saved)));
                body.push(
                    vcx.mk_assert_stmt(vcx.mk_eq_expr(revealed_sequence_state(target, next), tail)),
                );
                let statements = vcx.alloc_slice(&[
                    vcx.mk_local_decl_stmt(frame_decl, Some(revealed_sequence_state(source, i))),
                    vcx.mk_if_stmt(vir::expr! { (i) < (length) }, vcx.alloc_slice(&body), &[]),
                    // Retain the aggregate equality as a proof fact before
                    // checking pointwise observations of the completed state.
                    vcx.mk_assert_stmt(vcx.mk_eq_expr(revealed_sequence_state(target, i), frame)),
                ]);
                let block = vcx.mk_cfg_block(
                    &vir::CfgBlockLabelData::Start,
                    &[],
                    statements,
                    &vir::TerminatorStmtGenData::Exit,
                );
                // Describe preservation directly in the method's pre-state.
                // Passing the same heap-dependent observer as an argument and
                // a precondition made callers prove equality across the
                // permission transfer itself, especially inside wand packages.
                let initial = vcx.mk_old_expr(sequence_state(source, i));
                let mut posts = vec![
                    typed(i),
                    resources(target, i),
                    vcx.mk_eq_expr(sequence_state(target, i), initial),
                ];
                let access = super::indexed::observe_element(raw_element(j), j);
                let observed = if packing {
                    vcx.mk_old_expr(state(false, j))
                } else {
                    state(false, j)
                };
                let offset = vcx
                    .mk_bin_op_expr(vir::BinOpKind::Sub, j, i)
                    .downcast_ty::<vir::Int>();
                // The aggregate postcondition already relates the completed
                // sequence to the pre-state. Observe individual unpacked
                // elements in that current sequence, avoiding a second
                // quantified comparison against the old group's heap map.
                let element_frame = if packing {
                    initial
                } else {
                    sequence_state(false, i)
                };
                let expected = vcx
                    .mk_seq_index_expr(element_frame, offset)
                    .downcast_ty::<vir::CSnap>();
                let triggers = vec![
                    vcx.mk_trigger(&[access.as_dyn()]),
                    vcx.mk_trigger(&[observed.as_dyn()]),
                ];
                posts.push(vcx.mk_forall_expr(
                    variables,
                    vcx.alloc_slice(&triggers),
                    vir::expr! { ([range(i)]) ==> ((observed) == (expected)) },
                ));
                let mut pres = vec![
                    vir::expr! { ((zero) <= (i)) && ((i) <= (length)) },
                    resources(source, i),
                ];
                if packing {
                    pres.push(typed(i));
                }
                methods.push(vcx.mk_method(
                    if packing { pack } else { unpack },
                    (sequence, start, params.ty_decls(), params.const_decls()),
                    &[],
                    vcx.alloc_slice(&pres),
                    vcx.alloc_slice(&posts),
                    Some(vcx.alloc_slice(&[block])),
                ));
            }
            Ok((
                methods,
                ResourceCollection {
                    pack,
                    unpack,
                    params,
                    raw_sequence: raw_observer,
                    element_state,
                },
            ))
        })
    }
    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for methods in Self::all_outputs_local_no_errors(program) {
            for method in methods {
                program.add_method(method);
            }
        }
    }
}

/// Observe a resource sequence without generating unused conversion methods.
pub(crate) struct ResourceSequenceStateEnc;

#[derive(Clone)]
pub(crate) struct ResourceSequenceState<'vir> {
    function: vir::FunctionIdn<'vir, Arguments, vir::Seq>,
    params: GenericParams<'vir>,
}

impl<'vir> ResourceSequenceState<'vir> {
    pub fn observe_elements<Curr, Next>(
        &self,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
    ) -> vir::ExprGenSeq<'vir, Curr, Next> {
        vir::with_vcx(|vcx| {
            self.function.call()(
                sequence,
                vcx.mk_uint::<0>().lazy(),
                vcx.alloc_slice(
                    &self
                        .params
                        .ty_exprs()
                        .iter()
                        .map(|e| e.lazy())
                        .collect::<Vec<_>>(),
                ),
                vcx.alloc_slice(
                    &self
                        .params
                        .const_exprs()
                        .iter()
                        .map(|e| e.lazy())
                        .collect::<Vec<_>>(),
                ),
            )
        })
    }
}

impl TaskEncoder for ResourceSequenceStateEnc {
    task_encoder::encoder_cache!(ResourceSequenceStateEnc);
    const ENCODER_NAME: &'static str = "resource sequence observer encoder";
    type TaskDescription<'vir> = RustTyDecomposition<'vir>;
    type OutputFullDependency<'vir> = ResourceSequenceState<'vir>;
    type OutputFullLocal<'vir> = vir::Function<'vir>;
    type EncodingError = String;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        ResourceCollectionEnc::normalize_element(task)
    }
    fn describe_error(error: String) -> String {
        error
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let conversion = deps.require_dep::<ResourceConversionEnc>(*task)?;
        if !conversion.supports_indexed_resources() {
            return Err(task_encoder::EncodeFullError::EncodingError(
                "nested collection resource observation is unsupported".into(),
                None,
            ));
        }
        let element_state = deps.require_dep::<ResourceElementStateEnc>(*task)?;
        let caster = deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
            (!task.ty.specifics.is_param()).then_some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: *task,
            }),
        )?;
        let params = deps.require_dep::<GenericParamsEnc>(task.args.context())?;
        vir::with_vcx(|vcx| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            format!("{task:?}").hash(&mut hash);
            let suffix = hash.finish();
            let seq_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
            let sequence = vcx.mk_local_decl("conversion_elements", seq_ty);
            let start = vcx.mk_local_decl("conversion_start", vir::TYPE_INT);
            let seq = vcx.mk_local_ex(sequence);
            let i = vcx.mk_local_ex(start);
            let length = vcx.mk_collection_len_expr(seq);
            let zero = vcx.mk_uint::<0>();
            let next = (vir::expr! { (i) + ([vcx.mk_uint::<1>()]) }).downcast_ty::<vir::Int>();
            let function = vir::FunctionIdn::<Arguments, vir::Seq>::new(
                vir::vir_format_identifier!(vcx, "resource_raw_sequence_{suffix:x}"),
                (seq_ty, vir::TYPE_INT, params.ty_args(), params.const_args()),
                vcx.mk_ty_seq(state_type()),
            );
            let binder = vcx.mk_local_decl("conversion_index", vir::TYPE_INT);
            let j = vcx.mk_local_ex(binder);
            let variables = vcx.alloc_slice(&[binder.as_dyn()]);
            let element =
                caster.cast_to_caller_ctx(vcx.mk_seq_index_expr(seq, j).downcast_ty::<vir::Snap>());
            let resources = quantify_resources(
                vcx,
                variables,
                &[],
                vir::expr! { ((i) <= (j)) && ((j) < (length)) },
                conversion.indexed_resources(element, j),
            );
            let body = vcx.mk_ternary_expr(
                vcx.mk_eq_expr(i, length),
                vcx.mk_seq_literal_expr(&[], state_type()),
                vcx.mk_seq_concat_expr(
                    vcx.mk_seq_literal_expr(
                        vcx.alloc_slice(&[element_state.observe(seq, i)]),
                        state_type(),
                    ),
                    function.call()(seq, next, params.ty_exprs(), params.const_exprs()),
                ),
            );
            let result = vcx.mk_result(vcx.mk_ty_seq(state_type()));
            let extent = vcx
                .mk_bin_op_expr(vir::BinOpKind::Sub, length, i)
                .downcast_ty::<vir::Int>();
            let result_element = vcx.mk_seq_index_expr(result, j).downcast_ty::<vir::CSnap>();
            let absolute_index = vcx
                .mk_bin_op_expr(vir::BinOpKind::Add, j, i)
                .downcast_ty::<vir::Int>();
            let observed = element_state.observe(seq, absolute_index);
            let elements = vcx.mk_forall_expr(
                variables, vcx.alloc_slice(&[vcx.mk_trigger(&[result_element.as_dyn()])]),
                vir::expr! { (((zero) <= (j)) && ((j) < (extent))) ==> ((result_element) == (observed)) },
            );
            let definition = vcx.opaque_function(vcx.mk_function(
                function,
                (sequence, start, params.ty_decls(), params.const_decls()),
                vcx.alloc_slice(&[
                    vir::expr! { ((zero) <= (i)) && ((i) <= (length)) },
                    resources,
                ]),
                vcx.alloc_slice(&[
                    vcx.mk_eq_expr(vcx.mk_collection_len_expr(result), extent),
                    elements,
                ]),
                None,
                Some(body),
            ));
            Ok((definition, ResourceSequenceState { function, params }))
        })
    }
    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for function in Self::all_outputs_local_no_errors(program) {
            program.add_function(function);
        }
    }
}

pub(crate) struct ResourceElementStateEnc;

#[derive(Clone)]
pub(crate) struct ResourceElementState<'vir> {
    raw_state: vir::FunctionIdn<'vir, Arguments, vir::CSnap>,
    group_state: vir::FunctionIdn<'vir, Arguments, vir::CSnap>,
    params: GenericParams<'vir>,
    opaque: Option<ResourceUse<'vir>>,
}

impl<'vir> ResourceElementState<'vir> {
    pub fn observe<Curr, Next>(
        &self,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
        index: vir::ExprGenInt<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        if let Some(resource) = self.opaque {
            return resource.sequence_element_state(sequence, index);
        }
        self.call(self.raw_state, sequence, index)
    }

    fn observe_group<Curr, Next>(
        &self,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
        index: vir::ExprGenInt<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        self.call(self.group_state, sequence, index)
    }

    fn call<Curr, Next>(
        &self,
        function: vir::FunctionIdn<'vir, Arguments, vir::CSnap>,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
        index: vir::ExprGenInt<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        vir::with_vcx(|vcx| {
            function.call()(
                sequence,
                index,
                vcx.alloc_slice(
                    &self
                        .params
                        .ty_exprs()
                        .iter()
                        .map(|e| e.lazy())
                        .collect::<Vec<_>>(),
                ),
                vcx.alloc_slice(
                    &self
                        .params
                        .const_exprs()
                        .iter()
                        .map(|e| e.lazy())
                        .collect::<Vec<_>>(),
                ),
            )
        })
    }
}

impl TaskEncoder for ResourceElementStateEnc {
    task_encoder::encoder_cache!(ResourceElementStateEnc);
    const ENCODER_NAME: &'static str = "indexed resource element observer encoder";
    type TaskDescription<'vir> = RustTyDecomposition<'vir>;
    type OutputFullDependency<'vir> = ResourceElementState<'vir>;
    type OutputFullLocal<'vir> = Vec<vir::Function<'vir>>;
    type EncodingError = String;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        ResourceCollectionEnc::normalize_element(task)
    }
    fn describe_error(error: String) -> String {
        error
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        deps.require_ref::<super::resources::ResourceSequenceEnc>(())?;
        let conversion = deps.require_dep::<ResourceConversionEnc>(*task)?;
        if !conversion.supports_indexed_resources() {
            return Err(task_encoder::EncodeFullError::EncodingError(
                "nested collection resource observation is unsupported".into(),
                None,
            ));
        }
        let caster = deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
            (!task.ty.specifics.is_param()).then_some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: *task,
            }),
        )?;
        let params = deps.require_dep::<GenericParamsEnc>(task.args.context())?;
        let opaque = matches!(
            task.ty.specifics,
            super::TySpecifics::Param(_) | super::TySpecifics::Opaque(_)
        )
        .then(|| ResourceUse::new(deps, *task))
        .transpose()?;
        vir::with_vcx(|vcx| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            format!("{task:?}").hash(&mut hash);
            let suffix = hash.finish();
            let seq_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
            let sequence = vcx.mk_local_decl("conversion_elements", seq_ty);
            let index = vcx.mk_local_decl("conversion_start", vir::TYPE_INT);
            let seq = vcx.mk_local_ex(sequence);
            let i = vcx.mk_local_ex(index);
            let zero = vcx.mk_uint::<0>();
            let length = vcx.mk_collection_len_expr(seq);
            let raw = vcx.mk_seq_index_expr(seq, i).downcast_ty::<vir::PSnap>();
            let value = caster.cast_to_caller_ctx(raw.upcast_ty::<vir::Snap>());
            let arity = (seq_ty, vir::TYPE_INT, params.ty_args(), params.const_args());
            let raw_state = vir::FunctionIdn::<Arguments, vir::CSnap>::new(
                vir::vir_format_identifier!(vcx, "resource_raw_state_{suffix:x}"),
                arity,
                state_type(),
            );
            let group_state = vir::FunctionIdn::<Arguments, vir::CSnap>::new(
                vir::vir_format_identifier!(vcx, "resource_group_state_{suffix:x}"),
                arity,
                state_type(),
            );
            let functions = [false, true]
                .into_iter()
                .map(|group| {
                    let permission = if group {
                        conversion.grouped_generic(raw, i)
                    } else {
                        conversion.indexed_resources(value, i)
                    };
                    let state = if group {
                        conversion.grouped_generic_state(raw, i)
                    } else {
                        conversion.observe_indexed(value, i)
                    };
                    vcx.mk_function(
                        if group { group_state } else { raw_state },
                        (sequence, index, params.ty_decls(), params.const_decls()),
                        vcx.alloc_slice(&[
                            vir::expr! { ((zero) <= (i)) && ((i) < (length)) },
                            permission,
                        ]),
                        &[],
                        None,
                        Some(state),
                    )
                })
                .collect();
            Ok((
                functions,
                ResourceElementState {
                    raw_state,
                    group_state,
                    params,
                    opaque,
                },
            ))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for functions in Self::all_outputs_local_no_errors(program) {
            for function in functions {
                program.add_function(function);
            }
        }
    }
}

/// Checked conversion from the observer's logical state sequence to the
/// generic snapshots stored by a Rust `Seq<ResourceState>`.
pub(crate) struct ResourceStateBoxSequenceEnc;

impl TaskEncoder for ResourceStateBoxSequenceEnc {
    task_encoder::encoder_cache!(ResourceStateBoxSequenceEnc);
    const ENCODER_NAME: &'static str = "resource state sequence snapshot encoder";
    type TaskDescription<'vir> = RustTyDecomposition<'vir>;
    type OutputFullDependency<'vir> = vir::FunctionIdn<'vir, vir::Seq, vir::Seq>;
    type OutputFullLocal<'vir> = vir::Function<'vir>;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        ResourceCollectionEnc::normalize_element(task)
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let caster =
            deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(Some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: *task,
            }))?;
        vir::with_vcx(|vcx| {
            let input_ty = vcx.mk_ty_seq(state_type());
            let output_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
            let function = vir::FunctionIdn::<vir::Seq, vir::Seq>::new(
                vir::ViperIdent::new("resource_state_box_sequence"),
                input_ty,
                output_ty,
            );
            let input = vcx.mk_local_decl("resource_states", input_ty);
            let states = vcx.mk_local_ex(input);
            let length = vcx.mk_collection_len_expr(states);
            let result = vcx.mk_result(output_ty);
            let index = vcx.mk_local_decl("state_index", vir::TYPE_INT);
            let j = vcx.mk_local_ex(index);
            let zero = vcx.mk_uint::<0>();
            let boxed = |i| {
                caster
                    .cast_to_callee_ctx(vcx.mk_seq_index_expr(states, i).downcast_ty::<vir::Snap>())
                    .downcast_ty::<vir::PSnap>()
            };
            let result_element = vcx.mk_seq_index_expr(result, j).downcast_ty::<vir::PSnap>();
            let elements = vcx.mk_forall_expr(
                vcx.alloc_slice(&[index.as_dyn()]),
                vcx.alloc_slice(&[vcx.mk_trigger(&[result_element.as_dyn()])]),
                vir::expr! { (((zero) <= (j)) && ((j) < (length))) ==> ((result_element) == ([boxed(j)])) },
            );
            let body = vcx.mk_ternary_expr(
                vcx.mk_eq_expr(length, zero),
                vcx.mk_seq_literal_expr(&[], vir::TYPE_PSNAP),
                vcx.mk_seq_concat_expr(
                    vcx.mk_seq_literal_expr(vcx.alloc_slice(&[boxed(zero)]), vir::TYPE_PSNAP),
                    function.call()(vcx.mk_seq_drop_expr(states, vcx.mk_uint::<1>())),
                ),
            );
            Ok((
                vcx.opaque_function(vcx.mk_function(
                    function,
                    (input,),
                    &[],
                    vcx.alloc_slice(&[
                        vcx.mk_eq_expr(vcx.mk_collection_len_expr(result), length),
                        elements,
                    ]),
                    None,
                    Some(body),
                )),
                function,
            ))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for function in Self::all_outputs_local_no_errors(program) {
            program.add_function(function);
        }
    }
}
