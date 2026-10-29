//! Checked recovery of element type tags from owned array or slice storage.
use super::{
    RustTyDecomposition, RustTyNormalized, TySpecifics,
    generics::{GArgsCastEnc, GArgsTyEnc, GenericParams, GenericParamsEnc},
    use_impure::TyUseImpureEnc,
    use_pure::TyUsePureEnc,
};
use std::hash::{Hash, Hasher};
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

// Only slices need an explicit extent. Fixed arrays retain their const-generic
// bound in both the helper signature and its checked recursive body.
type Arguments = (
    vir::Ref,
    vir::Int,
    vir::ManyInt,
    vir::ManyTyVal,
    vir::ManyCSnap,
);

pub(crate) struct ArrayTypingEnc;

#[derive(Clone)]
pub(crate) struct ArrayTyping<'vir> {
    method: vir::MethodIdn<'vir, Arguments>,
    params: GenericParams<'vir>,
    extent: Option<vir::ExprInt<'vir>>,
}

impl<'vir> ArrayTyping<'vir> {
    pub fn establish(&self, storage: vir::ExprRef<'vir>) -> vir::Stmt<'vir> {
        self.establish_extent(storage, self.extent.expect("fixed array extent"))
    }

    pub fn establish_extent(
        &self,
        storage: vir::ExprRef<'vir>,
        extent: vir::ExprInt<'vir>,
    ) -> vir::Stmt<'vir> {
        vir::with_vcx(|vcx| {
            (self.method)(
                storage,
                vcx.mk_uint::<0>(),
                vcx.alloc_slice(&if self.extent.is_none() {
                    vec![extent]
                } else {
                    vec![]
                }),
                self.params.ty_exprs(),
                self.params.const_exprs(),
            )
            .alloc()
        })
    }
}

impl TaskEncoder for ArrayTypingEnc {
    task_encoder::encoder_cache!(ArrayTypingEnc);
    const ENCODER_NAME: &'static str = "array element typing encoder";
    type TaskDescription<'vir> = RustTyDecomposition<'vir>;
    type OutputFullDependency<'vir> = ArrayTyping<'vir>;
    type OutputFullLocal<'vir> = vir::Method<'vir>;
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
        let physical = deps.require_dep::<TyUseImpureEnc>(*task)?;
        let pure = deps.require_dep::<TyUsePureEnc>(*task)?;
        let caster = deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
            (!element.ty.specifics.is_param()).then_some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: element,
            }),
        )?;
        let params = deps.require_dep::<GenericParamsEnc>(task.args.context())?;
        let args = deps.require_dep::<GArgsTyEnc>(task.args)?;
        let usize_ty = deps.require_dep::<TyUsePureEnc>(vir::with_vcx(|vcx| {
            RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize)
        }))?;
        vir::with_vcx(|vcx| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            format!("{task:?}").hash(&mut hash);
            let method = vir::MethodIdn::<Arguments>::new(
                vir::vir_format_identifier!(vcx, "array_element_typing_{:x}", hash.finish()),
                (
                    vir::TYPE_REF,
                    vir::TYPE_INT,
                    vcx.alloc_slice(&if data.slice {
                        vec![vir::TYPE_INT]
                    } else {
                        vec![]
                    }),
                    params.ty_args(),
                    params.const_args(),
                ),
            );
            let storage_decl = vcx.mk_local_decl("typing_storage", vir::TYPE_REF);
            let start_decl = vcx.mk_local_decl("typing_start", vir::TYPE_INT);
            let extent_decl = vcx.mk_local_decl("typing_extent", vir::TYPE_INT);
            let storage = vcx.mk_local_ex(storage_decl);
            let start = vcx.mk_local_ex(start_decl);
            let snapshot = physical.ref_to_snap(storage).downcast_ty::<vir::CSnap>();
            let before_decl = vcx.mk_local_decl("typing_before", snapshot.ty());
            let before = vcx.mk_local_ex(before_decl);
            let fixed_extent = (!data.slice).then(|| {
                usize_ty
                    .expect_primitive()
                    .snap_to_prim(args.get_const()[0])
                    .downcast_ty::<vir::Int>()
            });
            let extent = fixed_extent.unwrap_or_else(|| vcx.mk_local_ex(extent_decl));
            let extent_args = vcx.alloc_slice(&if data.slice { vec![extent] } else { vec![] });
            let extent_decls = vcx.alloc_slice(&if data.slice {
                vec![extent_decl]
            } else {
                vec![]
            });
            let zero = vcx.mk_uint::<0>();
            let binder = vcx.mk_local_decl("typing_index", vir::TYPE_INT);
            let j = vcx.mk_local_ex(binder);
            let elements = |snap| pure.expect_array().to_seq(snap, extent);
            let raw = |snap, index| {
                vcx.mk_seq_index_expr(elements(snap), index)
                    .downcast_ty::<vir::PSnap>()
            };
            let normalized = |snap, index| {
                let value = raw(snap, index).upcast_ty::<vir::Snap>();
                vcx.mk_eq_expr(
                    caster.cast_to_callee_ctx(caster.cast_to_caller_ctx(value)),
                    value,
                )
            };
            let mut body = physical.unfold(None, storage, Some(start), None, None);
            body.push(vcx.mk_assert_stmt(normalized(before, start)));
            body.extend(physical.fold(None, storage, Some(start), None, None));
            body.push(vcx.mk_assert_stmt(vcx.mk_eq_expr(snapshot, before)));
            body.push(
                (method)(
                    storage,
                    (vir::expr! { (start) + ([vcx.mk_uint::<1>()]) }).downcast_ty(),
                    extent_args,
                    params.ty_exprs(),
                    params.const_exprs(),
                )
                .alloc(),
            );
            let statements = vcx.alloc_slice(&[
                vcx.mk_local_decl_stmt(before_decl, Some(snapshot)),
                vcx.mk_if_stmt(
                    vir::expr! { (start) < (extent) },
                    vcx.alloc_slice(&body),
                    &[],
                ),
            ]);
            let block = vcx.mk_cfg_block(
                &vir::CfgBlockLabelData::Start,
                &[],
                statements,
                &vir::TerminatorStmtGenData::Exit,
            );
            let predicate = physical.ref_to_pred(vcx, storage, None);
            let typed = vcx.mk_forall_expr(
                vcx.alloc_slice(&[binder.as_dyn()]),
                vcx.alloc_slice(&[vcx.mk_trigger(&[raw(snapshot, j).as_dyn()])]),
                vir::expr! { (((start) <= (j)) && ((j) < (extent))) ==> ([normalized(snapshot, j)]) },
            );
            Ok((
                vcx.mk_method(
                    method,
                    (
                        storage_decl,
                        start_decl,
                        extent_decls,
                        params.ty_decls(),
                        params.const_decls(),
                    ),
                    &[],
                    vcx.alloc_slice(&[
                        predicate,
                        vir::expr! { ((zero) <= (start)) && ((start) <= (extent)) },
                    ]),
                    vcx.alloc_slice(&[
                        predicate,
                        vcx.mk_eq_expr(snapshot, vcx.mk_old_expr(snapshot)),
                        typed,
                    ]),
                    Some(vcx.alloc_slice(&[block])),
                ),
                ArrayTyping {
                    method,
                    params,
                    extent: fixed_extent,
                },
            ))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for method in Self::all_outputs_local_no_errors(program) {
            program.add_method(method);
        }
    }
}
