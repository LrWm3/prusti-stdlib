//! Whole indirect-resource groups for opaque generic values.
//!
//! The group is separate from physical storage. Known type constructors have
//! checked predicate bodies; a type without a known body retains an opaque
//! predicate. Collection wrappers distinguish equal element snapshots.
use rustc_hash::FxHashMap;
use task_encoder::{EncodeFullError, EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::{CastType, Foldable, Folder};

use super::{
    RustTy, RustTyDecomposition, RustTyNormalized, TySpecifics,
    generics::{GArgCaster, GArgsCastEnc, GArgsTyEnc, GenericParamsEnc, ParamTypEnc, TyExprEnc},
    indexed::{IndexedResources, quantify_resources},
    lifted::TyConstructorEnc,
    use_impure::TyUseImpureEnc,
    use_pure::TyUsePureEnc,
};
use crate::encoders::Pure;

pub(crate) struct ResourceGroupEnc;

#[derive(Clone)]
pub(crate) struct ResourceCase<'vir> {
    guard: vir::ExprBool<'vir>,
    body: vir::ExprBool<'vir>,
}

pub(crate) fn group<'vir>(
    snap: vir::ExprPSnap<'vir>,
    ty: vir::ExprTyVal<'vir>,
    metadata: vir::ExprPSnap<'vir>,
) -> vir::ExprBool<'vir> {
    application("indirect_resources", snap, ty, metadata)
}

fn application<'vir>(
    name: &'vir str,
    snap: vir::ExprPSnap<'vir>,
    ty: vir::ExprTyVal<'vir>,
    metadata: vir::ExprPSnap<'vir>,
) -> vir::ExprBool<'vir> {
    vir::with_vcx(|vcx| {
        vcx.mk_predicate_app_expr(vcx.alloc(vir::PredicateAppData {
            target: name,
            args: vcx.alloc_slice(&[snap.as_dyn(), ty.as_dyn(), metadata.as_dyn()]),
            perm: None,
        }))
    })
}

fn declarations<'vir>() -> (
    vir::LocalDeclPSnap<'vir>,
    vir::LocalDeclTyVal<'vir>,
    vir::LocalDeclPSnap<'vir>,
) {
    vir::with_vcx(|vcx| {
        (
            vcx.mk_local_decl("resource_snapshot", vir::TYPE_PSNAP),
            vcx.mk_local_decl("resource_type", vir::TYPE_TYVAL),
            vcx.mk_local_decl("resource_metadata", vir::TYPE_PSNAP),
        )
    })
}

/// A use preserves the caller's snapshot/type substitution. Requesting a
/// reference to the constructor task breaks recursive type dependencies.
#[derive(Clone, Copy)]
pub(crate) struct ResourceUse<'vir> {
    caster: GArgCaster<'vir, Pure>,
    ty: vir::ExprTyVal<'vir>,
}
impl<'vir> ResourceUse<'vir> {
    pub fn new<E: TaskEncoder>(
        deps: &mut TaskEncoderDependencies<'vir, E>,
        ty: RustTyDecomposition<'vir>,
    ) -> Result<Self, EncodeFullError<'vir, E>> {
        deps.require_ref::<ResourceGroupEnc>(ty.ty)?;
        let normalized = (!ty.ty.specifics.is_param()).then_some(RustTyNormalized {
            param: RustTyDecomposition::param(),
            concrete: ty,
        });
        Ok(Self {
            caster: deps.require_dep::<GArgsCastEnc<Pure>>(normalized)?,
            ty: deps.require_dep::<TyExprEnc>(ty)?,
        })
    }
    pub fn apply(
        self,
        snap: vir::ExprSnap<'vir>,
        metadata: vir::ExprPSnap<'vir>,
    ) -> vir::ExprBool<'vir> {
        group(
            self.caster.cast_to_callee_ctx(snap).downcast_ty(),
            self.ty,
            metadata,
        )
    }
}

pub(crate) fn unit_metadata<'vir, E: TaskEncoder>(
    deps: &mut TaskEncoderDependencies<'vir, E>,
) -> Result<vir::ExprPSnap<'vir>, EncodeFullError<'vir, E>> {
    vir::with_vcx(|vcx| {
        let unit =
            RustTyDecomposition::from_ty(vcx.tcx().types.unit, super::generics::GParams::empty());
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

impl TaskEncoder for ResourceGroupEnc {
    task_encoder::encoder_cache!(ResourceGroupEnc);
    const ENCODER_NAME: &'static str = "whole indirect resource encoder";
    type TaskDescription<'vir> = RustTy<'vir>;
    type OutputFullLocal<'vir> = Option<ResourceCase<'vir>>;
    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        *task
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        if matches!(
            task.specifics,
            TySpecifics::Param(_) | TySpecifics::Opaque(_)
        ) {
            return Ok((None, ()));
        }
        vir::with_vcx(|vcx| {
            let identity = RustTyDecomposition::identity(*task);
            let params = deps.require_dep::<GenericParamsEnc>(task.params)?;
            let constructor = deps.require_ref::<TyConstructorEnc>(*task)?;
            let caster = deps.require_dep::<GArgsCastEnc<Pure>>(Some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: identity,
            }))?;
            let (s, t, m) = declarations();
            let ty_expr = vcx.mk_local_ex(t);
            let snap = caster.cast_to_caller_ctx(vcx.mk_local_ex(s).upcast_ty());
            let metadata = vcx.mk_local_ex(m);
            let pure = deps.require_dep::<TyUsePureEnc>(identity)?;
            let combined = task.zip(pure);
            let unit = unit_metadata(deps)?;
            let typ = deps.require_dep::<ParamTypEnc>(())?.typ;
            let mut resources = vec![vcx.mk_eq_expr(typ(vcx.mk_local_ex(s)), ty_expr)];
            match combined.specifics {
                TySpecifics::Primitive(_)
                | TySpecifics::ImmRef(_)
                | TySpecifics::Raw(_)
                | TySpecifics::Builtin(_) => (),
                TySpecifics::Param(_) | TySpecifics::Opaque(_) => unreachable!(),
                TySpecifics::MutRef((data, reference)) => {
                    let referent = data.referent.decompose(task.params);
                    let physical = deps.require_dep::<TyUseImpureEnc>(referent)?;
                    let address = reference.deref_access(snap.downcast_ty());
                    resources.push(physical.ref_to_pred(vcx, address, None));
                    let nested = ResourceUse::new(deps, referent)?;
                    resources.push(nested.apply(
                        physical.ref_to_snap(address),
                        reference.generic_metadata(snap.downcast_ty()),
                    ));
                }
                TySpecifics::StructLike(data) => {
                    for (field, accessor) in data.fields {
                        let resource = ResourceUse::new(deps, field.ty().decompose(task.params))?;
                        resources.push(resource.apply(accessor.read(snap.downcast_ty()), unit));
                    }
                }
                TySpecifics::EnumLike(data) => {
                    let discr =
                        pure.expect_enumlike().snap_to_discr_snap.call()(snap.downcast_ty());
                    for variant in data.variants {
                        let mut fields = vec![];
                        for (field, accessor) in variant.inner.fields {
                            fields.push(
                                ResourceUse::new(deps, field.ty().decompose(task.params))?
                                    .apply(accessor.read(snap.downcast_ty()), unit),
                            );
                        }
                        let guard = vcx.mk_eq_expr(discr, variant.data.1.discr);
                        let body = vcx.mk_conj(&fields);
                        resources.push(vir::expr! { (guard) ==> (body) });
                    }
                }
                TySpecifics::ArrayLike(data) => {
                    let element = data.0.decompose(task.params);
                    let resource = ResourceUse::new(deps, element)?;
                    let usize_ty = RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize);
                    let usize_pure = deps.require_dep::<TyUsePureEnc>(usize_ty)?;
                    let extent = if data.slice {
                        let caster =
                            deps.require_dep::<GArgsCastEnc<Pure>>(Some(RustTyNormalized {
                                param: RustTyDecomposition::param(),
                                concrete: usize_ty,
                            }))?;
                        caster
                            .cast_to_caller_ctx(metadata.upcast_ty())
                            .downcast_ty()
                    } else {
                        deps.require_dep::<GArgsTyEnc>(identity.args)?.get_const()[0]
                    };
                    let extent = usize_pure
                        .expect_primitive()
                        .snap_to_prim(extent)
                        .downcast_ty::<vir::Int>();
                    let index_decl = vcx.mk_local_decl("resource_index", vir::TYPE_INT);
                    let index = vcx.mk_local_ex(index_decl);
                    let raw =
                        resource.apply(pure.expect_array().index(snap.downcast_ty(), index), unit);
                    let indexed = IndexedResources::new(deps, raw)?;
                    let wrapped = indexed.wrap(raw, index, &[]);
                    let zero = vcx.mk_uint::<0>();
                    resources.push(quantify_resources(
                        vcx,
                        vcx.alloc_slice(&[index_decl.as_dyn()]),
                        &[],
                        vir::expr! { ((zero) <= (index)) && ((index) < (extent)) },
                        wrapped,
                    ));
                }
            }
            let tys = constructor
                .ty_param_accessors
                .iter()
                .map(|a| a.call()(ty_expr))
                .collect::<Vec<_>>();
            let consts = constructor
                .const_param_accessors
                .iter()
                .map(|a| a.call()(ty_expr))
                .collect::<Vec<_>>();
            let guard = vcx.mk_eq_expr(ty_expr, constructor.ty_constructor.call()(&tys, &consts));
            let substitutions = params
                .ty_decls()
                .iter()
                .map(|d| d.name)
                .zip(tys.iter().map(|e| e.as_dyn()))
                .chain(
                    params
                        .const_decls()
                        .iter()
                        .map(|d| d.name)
                        .zip(consts.iter().map(|e| e.as_dyn())),
                )
                .collect::<FxHashMap<_, _>>();
            struct Substitute<'a, 'vir>(&'a FxHashMap<&'vir str, vir::ExprDyn<'vir>>);
            impl<'vir> Folder<'vir, (), !> for Substitute<'_, 'vir> {
                fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                    if let vir::ExprKindData::Local(local) = e.kind {
                        self.0.get(local.name).copied()
                    } else {
                        e.super_fold_with(self)
                    }
                }
            }
            let body = vcx.mk_conj(&resources);
            let body = body
                .fold_with(&mut Substitute(&substitutions))
                .unwrap_or(body);
            Ok((Some(ResourceCase { guard, body }), ()))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        let cases = Self::all_outputs_local_no_errors(program)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if cases.is_empty() && Self::all_outputs_local_no_errors(program).is_empty() {
            return;
        }
        vir::with_vcx(|vcx| {
            let (s, t, m) = declarations();
            let params = vcx.alloc_slice(&[s.as_dyn(), t.as_dyn(), m.as_dyn()]);
            let opaque = application(
                "opaque_indirect_resources",
                vcx.mk_local_ex(s),
                vcx.mk_local_ex(t),
                vcx.mk_local_ex(m),
            );
            let body = cases.iter().fold(opaque, |tail, case| {
                vcx.mk_ternary_expr(case.guard, case.body, tail)
            });
            program.add_predicate(vcx.mk_predicate_unchecked(
                "opaque_indirect_resources",
                params,
                None,
            ));
            program.add_predicate(vcx.mk_predicate_unchecked(
                "indirect_resources",
                params,
                Some(body),
            ));
        });
    }
}
