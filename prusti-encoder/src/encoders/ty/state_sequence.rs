//! Checked immutable maps from stored values to resource observation states.
//! Ownership is supplied separately by the caller's referent segment.

use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

use super::{
    RustTyDecomposition, RustTyNormalized,
    generics::GArgsCastEnc,
    resource_observation::{ResourceStateEnc, state_type},
};

pub(crate) struct StateSequenceEnc;

#[derive(Clone)]
pub(crate) struct StateSequence<'vir> {
    pub references: vir::FunctionIdn<'vir, vir::Seq, vir::Seq>,
    pub empty: vir::FunctionIdn<'vir, vir::Seq, vir::Seq>,
    pub unresolved: vir::FunctionIdn<'vir, (vir::Seq, vir::TyVal), vir::Seq>,
}

#[derive(Clone)]
pub(crate) struct Definitions<'vir> {
    functions: Vec<vir::Function<'vir>>,
    unresolved: vir::Predicate<'vir>,
}

impl TaskEncoder for StateSequenceEnc {
    task_encoder::encoder_cache!(StateSequenceEnc);
    const ENCODER_NAME: &'static str = "checked resource state sequence map";
    // The concrete ResourceState type, used only for the ordinary Seq<T> box.
    type TaskDescription<'vir> = RustTyDecomposition<'vir>;
    type OutputFullDependency<'vir> = StateSequence<'vir>;
    type OutputFullLocal<'vir> = Definitions<'vir>;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        *task
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        deps.require_dep::<ResourceStateEnc>(())?;
        let caster =
            deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(Some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: *task,
            }))?;
        vir::with_vcx(|vcx| {
            let seq_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
            let references = vir::FunctionIdn::new(
                vir::ViperIdent::new("reference_resource_states"),
                seq_ty,
                seq_ty,
            );
            let empty = vir::FunctionIdn::new(
                vir::ViperIdent::new("empty_resource_states"),
                seq_ty,
                seq_ty,
            );
            let unresolved = vir::FunctionIdn::new(
                vir::ViperIdent::new("unresolved_resource_states"),
                (seq_ty, vir::TYPE_TYVAL),
                seq_ty,
            );
            let input = vcx.mk_local_decl("stored_values", seq_ty);
            let values = vcx.mk_local_ex(input);
            let length = vcx.mk_collection_len_expr(values);
            let zero = vcx.mk_int::<0>();
            let result = vcx.mk_result(seq_ty);
            let index = vcx.mk_local_decl("state_index", vir::TYPE_INT);
            let j = vcx.mk_local_ex(index);
            let empty_state = vir::FunctionIdn::<(), vir::CSnap>::new(
                vir::ViperIdent::new("ResourceEmpty"),
                (),
                state_type(),
            )
            .call()();
            let reference = vir::FunctionIdn::<(vir::PSnap, vir::CSnap), vir::CSnap>::new(
                vir::ViperIdent::new("ResourceReference"),
                (vir::TYPE_PSNAP, state_type()),
                state_type(),
            );
            let mut functions = Vec::new();
            for (function, is_reference) in [(references, true), (empty, false)] {
                let boxed = |i| {
                    let state = if is_reference {
                        reference.call()(
                            vcx.mk_seq_index_expr(values, i).downcast_ty(),
                            empty_state,
                        )
                    } else {
                        empty_state
                    };
                    caster
                        .cast_to_callee_ctx(state.upcast_ty())
                        .downcast_ty::<vir::PSnap>()
                };
                let selected = vcx.mk_seq_index_expr(result, j).downcast_ty::<vir::PSnap>();
                let elements = vcx.mk_forall_expr(
                    vcx.alloc_slice(&[index.as_dyn()]),
                    vcx.alloc_slice(&[vcx.mk_trigger(&[selected.as_dyn()])]),
                    vir::expr! { (((zero) <= (j)) && ((j) < (length))) ==> ((selected) == ([boxed(j)])) },
                );
                let body = vcx.mk_ternary_expr(
                    vcx.mk_eq_expr(length, zero),
                    vcx.mk_seq_literal_expr(&[], vir::TYPE_PSNAP),
                    vcx.mk_seq_concat_expr(
                        vcx.mk_seq_literal_expr(vcx.alloc_slice(&[boxed(zero)]), vir::TYPE_PSNAP),
                        function.call()(vcx.mk_seq_drop_expr(values, vcx.mk_int::<1>())),
                    ),
                );
                functions.push(vcx.opaque_function(vcx.mk_function(
                    function,
                    (input,),
                    &[],
                    vcx.alloc_slice(&[
                        vcx.mk_eq_expr(vcx.mk_collection_len_expr(result), length),
                        elements,
                    ]),
                    Some(vcx.alloc(vir::DecreasesGenData::Tuple(
                        vcx.alloc_slice(&[length.as_dyn()]),
                        None,
                    ))),
                    Some(body),
                )));
            }
            // No encoder operation grants this capability. Unsupported generic
            // state is not silently interpreted as empty state.
            let ty = vcx.mk_local_decl("element_type", vir::TYPE_TYVAL);
            let capability = vir::PredicateIdn::<(vir::Seq, vir::TyVal)>::new(
                vir::ViperIdent::new("unresolved_sequence_observation_resources"),
                (seq_ty, vir::TYPE_TYVAL),
            );
            let predicate = vcx.mk_predicate(capability, (input, ty), None);
            functions.push(vcx.mk_function(
                unresolved,
                (input, ty),
                vcx.alloc_slice(&[
                    vcx.mk_predicate_app_expr(capability(values, vcx.mk_local_ex(ty))(None)),
                ]),
                &[],
                None,
                None,
            ));
            Ok((
                Definitions {
                    functions,
                    unresolved: predicate,
                },
                StateSequence {
                    references,
                    empty,
                    unresolved,
                },
            ))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for output in Self::all_outputs_local_no_errors(program) {
            program.add_predicate(output.unresolved);
            for function in &output.functions {
                program.add_function(*function);
            }
        }
    }
}
