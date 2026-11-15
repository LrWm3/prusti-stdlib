//! Permission-checked value observations, separate from ownership transfers.
//! Unknown structures require an opaque capability which this module never grants.

use task_encoder::{EncodeFullError, EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

use super::{
    RustBuiltinData, RustTyDecomposition, RustTyNormalized, TySpecifics,
    generics::{GArgsCastEnc, GParams, TyExprEnc},
    use_impure::TyUseImpureEnc,
    use_pure::TyUsePureEnc,
};
use crate::encoders::Pure;

pub(crate) struct ResourceStateEnc;

pub(crate) fn state_type<'vir>() -> vir::TypeCSnap<'vir> {
    vir::DomainIdnSnap::new(vir::ViperIdent::new("s_ResourceState"), 0)().downcast_ty()
}

pub(crate) fn checked_index<'vir>()
-> vir::FunctionIdn<'vir, (vir::CSnap, vir::Int, vir::Int), vir::CSnap> {
    vir::FunctionIdn::new(
        vir::ViperIdent::new("checked_resource_observation_index"),
        (state_type(), vir::TYPE_INT, vir::TYPE_INT),
        state_type(),
    )
}

fn opaque_state<'vir>() -> vir::FunctionIdn<'vir, (vir::PSnap, vir::TyVal, vir::PSnap), vir::CSnap>
{
    vir::FunctionIdn::new(
        vir::ViperIdent::new("observed_opaque_resource_state"),
        (vir::TYPE_PSNAP, vir::TYPE_TYVAL, vir::TYPE_PSNAP),
        state_type(),
    )
}

impl TaskEncoder for ResourceStateEnc {
    task_encoder::encoder_cache!(ResourceStateEnc);
    const ENCODER_NAME: &'static str = "resource observation state domain";
    type TaskDescription<'vir> = ();

    fn task_to_key<'vir>(_: &()) -> Self::TaskKey<'vir> {}

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        Ok(((), ()))
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        if Self::all_outputs_local_no_errors(program).is_empty() {
            return;
        }
        vir::with_vcx(|vcx| {
            let empty = vcx.mk_adt_constructor("ResourceEmpty", &[] as &[vir::LocalDeclDyn<'vir>]);
            let reference = vcx.mk_adt_constructor(
                "ResourceReference",
                vcx.alloc_slice(&[
                    vcx.mk_local_decl("resource_stored", vir::TYPE_PSNAP)
                        .as_dyn(),
                    vcx.mk_local_decl("resource_nested", state_type()).as_dyn(),
                ]),
            );
            let unknown = vcx.mk_adt_constructor(
                "ResourceUnknown",
                vcx.alloc_slice(&[vcx
                    .mk_local_decl("resource_unknown", vir::TYPE_INT)
                    .as_dyn()]),
            );
            program.add_adt(vcx.mk_adt(
                vir::ViperIdent::new("s_ResourceState"),
                &[],
                vcx.alloc_slice(&[empty, reference, unknown]),
            ));
            let s = vcx.mk_local_decl("resource_snapshot", vir::TYPE_PSNAP);
            let t = vcx.mk_local_decl("resource_type", vir::TYPE_TYVAL);
            let m = vcx.mk_local_decl("resource_metadata", vir::TYPE_PSNAP);
            program.add_predicate(vcx.mk_predicate_unchecked(
                "unresolved_observation_resources",
                vcx.alloc_slice(&[s.as_dyn(), t.as_dyn(), m.as_dyn()]),
                None,
            ));
            let permission = vcx.mk_predicate_app_expr(vcx.alloc(vir::PredicateAppGenData {
                target: "unresolved_observation_resources",
                args: vcx.alloc_slice(&[
                    vcx.mk_local_ex(s).as_dyn(),
                    vcx.mk_local_ex(t).as_dyn(),
                    vcx.mk_local_ex(m).as_dyn(),
                ]),
                perm: None,
            }));
            program.add_function(vcx.mk_function(
                opaque_state(),
                (s, t, m),
                vcx.alloc_slice(&[permission]),
                &[],
                None,
                None,
            ));
            // Keep the bounds obligation even when the element has no indirect
            // resources and its state does not otherwise use the selected value.
            let state = vcx.mk_local_decl("observed_state", state_type());
            let index = vcx.mk_local_decl("observed_index", vir::TYPE_INT);
            let length = vcx.mk_local_decl("observed_length", vir::TYPE_INT);
            let i = vcx.mk_local_ex(index);
            let n = vcx.mk_local_ex(length);
            program.add_function(vcx.mk_function(
                checked_index(),
                (state, index, length),
                vcx.alloc_slice(&[
                    vir::expr! { ([vcx.mk_uint::<0>()]) <= ([i]) },
                    vir::expr! { ([i]) < ([n]) },
                ]),
                &[],
                None,
                Some(vcx.mk_local_ex(state)),
            ));
        });
    }
}

fn empty<'vir, Curr, Next>() -> vir::ExprGenCSnap<'vir, Curr, Next> {
    vir::FunctionIdn::<(), vir::CSnap>::new(vir::ViperIdent::new("ResourceEmpty"), (), state_type())
        .call()()
}

pub(crate) fn unit_metadata<'vir, E: TaskEncoder>(
    deps: &mut TaskEncoderDependencies<'vir, E>,
) -> Result<vir::ExprPSnap<'vir>, EncodeFullError<'vir, E>> {
    vir::with_vcx(|vcx| {
        let unit = RustTyDecomposition::from_ty(vcx.tcx().types.unit, GParams::empty());
        let value = deps
            .require_dep::<TyUsePureEnc>(unit)?
            .expect_structlike()
            .field_snaps_to_snap(vec![]);
        let caster = deps.require_dep::<GArgsCastEnc<Pure>>(Some(RustTyNormalized {
            param: RustTyDecomposition::param(),
            concrete: unit,
        }))?;
        Ok(caster.cast_to_callee_ctx(value.upcast_ty()).downcast_ty())
    })
}

pub(crate) fn observe<'vir, E: TaskEncoder, Curr, Next>(
    deps: &mut TaskEncoderDependencies<'vir, E>,
    ty: RustTyDecomposition<'vir>,
    snapshot: vir::ExprGenSnap<'vir, Curr, Next>,
    metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
) -> Result<vir::ExprGenCSnap<'vir, Curr, Next>, EncodeFullError<'vir, E>> {
    deps.require_dep::<ResourceStateEnc>(())?;
    match &ty.ty.specifics {
        TySpecifics::Primitive(_) | TySpecifics::ImmRef(_) | TySpecifics::Raw(_) => Ok(empty()),
        TySpecifics::Builtin(
            RustBuiltinData::ResourceState | RustBuiltinData::Int | RustBuiltinData::Real,
        ) => Ok(empty()),
        TySpecifics::StructLike(data) if data.fields.is_empty() => Ok(empty()),
        TySpecifics::MutRef(data) => {
            let reference = deps
                .require_dep::<TyUsePureEnc>(ty)?
                .expect_mutref()
                .clone();
            let physical = deps.require_dep::<TyUseImpureEnc>(
                data.referent.decompose_context(ty.ty.params, ty.args),
            )?;
            let address = reference.deref_access(snapshot.downcast_ty());
            // Keep the predicate-function read and its permission precondition.
            // A reference snapshot alone never grants access to this value.
            let stored = physical.ref_to_snap(address).downcast_ty();
            let nested = observe(
                deps,
                data.referent.decompose_normalize(ty.args),
                reference.cast_to_caller_ctx(stored),
                reference.generic_metadata(snapshot.downcast_ty()),
            )?;
            Ok(
                vir::FunctionIdn::<(vir::PSnap, vir::CSnap), vir::CSnap>::new(
                    vir::ViperIdent::new("ResourceReference"),
                    (vir::TYPE_PSNAP, state_type()),
                    state_type(),
                )
                .call()(stored, nested),
            )
        }
        _ => {
            let normalized = (!ty.ty.specifics.is_param()).then_some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: ty,
            });
            let caster = deps.require_dep::<GArgsCastEnc<Pure>>(normalized)?;
            let encoded_ty = deps.require_dep::<TyExprEnc>(ty)?;
            Ok(opaque_state().call()(
                caster.cast_to_callee_ctx(snapshot).downcast_ty(),
                encoded_ty.lazy(),
                metadata,
            ))
        }
    }
}
