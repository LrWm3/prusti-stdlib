//! Checked whole-array boundaries with explicit resource-state preservation.
use super::{
    RustTyDecomposition, RustTyNormalized, TySpecifics,
    generics::{GArgsCastEnc, GArgsTyEnc, GenericParams, GenericParamsEnc},
    indexed::{observe_element, quantify_resources},
    resource_collection::{ResourceCollectionEnc, ResourceCollectionTask},
    resource_conversion::ResourceConversionEnc,
    resources::ResourceUse,
    use_pure::TyUsePureEnc,
};
use std::hash::{Hash, Hasher};
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

type Arguments = (vir::CSnap, vir::PSnap, vir::ManyTyVal, vir::ManyCSnap);
pub(crate) struct ResourceArrayEnc;

#[derive(Clone)]
pub(crate) struct ResourceArray<'vir> {
    pack: vir::MethodIdn<'vir, Arguments>,
    unpack: vir::MethodIdn<'vir, Arguments>,
    params: GenericParams<'vir>,
}

impl<'vir> ResourceArray<'vir> {
    pub fn statement(
        &self,
        snapshot: vir::ExprCSnap<'vir>,
        metadata: vir::ExprPSnap<'vir>,
        pack: bool,
    ) -> vir::Stmt<'vir> {
        (if pack { self.pack } else { self.unpack })(
            snapshot,
            metadata,
            self.params.ty_exprs(),
            self.params.const_exprs(),
        )
        .alloc()
    }
}

impl TaskEncoder for ResourceArrayEnc {
    task_encoder::encoder_cache!(ResourceArrayEnc);
    const ENCODER_NAME: &'static str = "whole array resource conversion encoder";
    type TaskDescription<'vir> = RustTyDecomposition<'vir>;
    type OutputFullDependency<'vir> = ResourceArray<'vir>;
    type OutputFullLocal<'vir> = Vec<vir::Method<'vir>>;
    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        *task
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let TySpecifics::ArrayLike(data) = &task.ty.specifics else {
            unreachable!()
        };
        let element = data.decompose_normalize(task.args);
        let conversion = deps.require_dep::<ResourceConversionEnc>(element)?;
        let collection =
            deps.require_dep::<ResourceCollectionEnc>(ResourceCollectionTask::aggregate(element))?;
        let pure = deps.require_dep::<TyUsePureEnc>(*task)?;
        let resource = ResourceUse::new(deps, *task)?;
        let params = deps.require_dep::<GenericParamsEnc>(task.args.context())?;
        let caster = deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
            (!element.ty.specifics.is_param()).then_some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: element,
            }),
        )?;
        let usize_decomp =
            vir::with_vcx(|vcx| RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize));
        let usize_ty = deps.require_dep::<TyUsePureEnc>(usize_decomp)?;
        let metadata_caster =
            deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(Some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: usize_decomp,
            }))?;
        let args = deps.require_dep::<GArgsTyEnc>(task.args)?;
        vir::with_vcx(|vcx| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            format!("{task:?}").hash(&mut hash);
            let suffix = hash.finish();
            let value_decl = vcx.mk_local_decl(
                "array_resource_value",
                pure.snapshot.downcast_ty::<vir::CSnap>(),
            );
            let metadata_decl = vcx.mk_local_decl("array_resource_metadata", vir::TYPE_PSNAP);
            let value = vcx.mk_local_ex(value_decl).downcast_ty::<vir::CSnap>();
            let metadata = vcx.mk_local_ex(metadata_decl);
            let extent = if data.slice {
                metadata_caster
                    .cast_to_caller_ctx(metadata.upcast_ty())
                    .downcast_ty()
            } else {
                args.get_const()[0]
            };
            let extent = usize_ty
                .expect_primitive()
                .snap_to_prim(extent)
                .downcast_ty::<vir::Int>();
            let seq = pure.expect_array().to_seq(value, extent);
            let index_decl = vcx.mk_local_decl("array_resource_index", vir::TYPE_INT);
            let index = vcx.mk_local_ex(index_decl);
            let variables = vcx.alloc_slice(&[index_decl.as_dyn()]);
            let zero = vcx.mk_uint::<0>();
            let range = vir::expr! { ((zero) <= (index)) && ((index) < (extent)) };
            let generic = vcx
                .mk_seq_index_expr(seq, index)
                .downcast_ty::<vir::PSnap>();
            let element = caster.cast_to_caller_ctx(generic.upcast_ty());
            let raw = quantify_resources(
                vcx,
                variables,
                &[],
                range,
                conversion.indexed_resources(element, index),
            );
            let typed = vcx.mk_forall_expr(variables, &[], vir::expr! {
                (range) ==> ([vcx.mk_eq_expr(caster.cast_to_callee_ctx(element), generic.upcast_ty::<vir::Snap>())])
            });
            let group = resource.apply(value.upcast_ty(), metadata);
            let vir::ExprKindData::PredicateApp(group_app) = group.kind else {
                unreachable!()
            };
            let raw_state = collection.observe(seq);
            let grouped_state = resource.state(value.upcast_ty(), metadata);
            let pack = vir::MethodIdn::<Arguments>::new(
                vir::vir_format_identifier!(vcx, "resource_array_pack_{suffix:x}"),
                (
                    value.ty(),
                    vir::TYPE_PSNAP,
                    params.ty_args(),
                    params.const_args(),
                ),
            );
            let unpack = vir::MethodIdn::<Arguments>::new(
                vir::vir_format_identifier!(vcx, "resource_array_unpack_{suffix:x}"),
                (
                    value.ty(),
                    vir::TYPE_PSNAP,
                    params.ty_args(),
                    params.const_args(),
                ),
            );
            let mut methods = Vec::new();
            for packing in [true, false] {
                let (source, target) = if packing {
                    (raw_state, grouped_state)
                } else {
                    (grouped_state, raw_state)
                };
                let saved_decl = vcx.mk_local_decl("array_resource_before", source.ty());
                let saved = vcx.mk_local_ex(saved_decl);
                let mut body = vec![vcx.mk_local_decl_stmt(saved_decl, Some(source))];
                if !packing {
                    body.push(vcx.mk_unfold_stmt(group_app));
                }
                body.extend(collection.statements(seq, packing));
                if packing {
                    body.push(vcx.mk_fold_stmt(group_app));
                }
                body.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(target, saved)));
                let mut posts = if packing {
                    vec![group]
                } else {
                    vec![raw, typed]
                };
                posts.push(vcx.mk_eq_expr(target, vcx.mk_old_expr(source)));
                // Export the checked element observations needed by callers
                // that read a concrete index after a generic call.
                let observed = collection.observe_element(seq, index);
                let observed = if packing {
                    vcx.mk_old_expr(observed)
                } else {
                    observed
                };
                let states = collection.observe_elements(seq);
                let states = if packing {
                    vcx.mk_old_expr(states)
                } else {
                    states
                };
                let expected = vcx
                    .mk_seq_index_expr(states, index)
                    .downcast_ty::<vir::CSnap>();
                let marker = observe_element(generic, index);
                posts.push(vcx.mk_forall_expr(
                    variables,
                    vcx.alloc_slice(&[
                        vcx.mk_trigger(&[marker.as_dyn()]),
                        vcx.mk_trigger(&[observed.as_dyn()]),
                    ]),
                    vir::expr! { (range) ==> ((observed) == (expected)) },
                ));
                let pres = if packing {
                    vec![vir::expr! { (zero) <= (extent) }, raw, typed]
                } else {
                    vec![group]
                };
                let block = vcx.mk_cfg_block(
                    &vir::CfgBlockLabelData::Start,
                    &[],
                    vcx.alloc_slice(&body),
                    &vir::TerminatorStmtGenData::Exit,
                );
                methods.push(vcx.mk_method(
                    if packing { pack } else { unpack },
                    (
                        value_decl,
                        metadata_decl,
                        params.ty_decls(),
                        params.const_decls(),
                    ),
                    &[],
                    vcx.alloc_slice(&pres),
                    vcx.alloc_slice(&posts),
                    Some(vcx.alloc_slice(&[block])),
                ));
            }
            Ok((
                methods,
                ResourceArray {
                    pack,
                    unpack,
                    params,
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
