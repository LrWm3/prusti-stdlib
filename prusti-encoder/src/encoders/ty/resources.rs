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
    state: Option<vir::ExprCSnap<'vir>>,
}

pub(crate) fn group<'vir, Curr, Next>(
    snap: vir::ExprGenPSnap<'vir, Curr, Next>,
    ty: vir::ExprGenTyVal<'vir, Curr, Next>,
    metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
) -> vir::ExprGenBool<'vir, Curr, Next> {
    application("indirect_resources", snap, ty, metadata)
}

fn application<'vir, Curr, Next>(
    name: &'vir str,
    snap: vir::ExprGenPSnap<'vir, Curr, Next>,
    ty: vir::ExprGenTyVal<'vir, Curr, Next>,
    metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
) -> vir::ExprGenBool<'vir, Curr, Next> {
    vir::with_vcx(|vcx| {
        vcx.mk_predicate_app_expr(vcx.alloc(vir::PredicateAppGenData {
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
    pub fn apply_generic<Curr, Next>(
        self,
        snapshot: vir::ExprGenPSnap<'vir, Curr, Next>,
        metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenBool<'vir, Curr, Next> {
        group(snapshot, self.ty.lazy(), metadata)
    }
    pub fn state_generic<Curr, Next>(
        self,
        snapshot: vir::ExprGenPSnap<'vir, Curr, Next>,
        metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        state_function("indirect_resource_state").call()(snapshot, self.ty.lazy(), metadata)
    }
    pub fn sequence_state<Curr, Next>(
        self,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
        start: vir::ExprGenInt<'vir, Curr, Next>,
    ) -> vir::ExprGenSeq<'vir, Curr, Next> {
        sequence_state_function().call()(sequence, self.ty.lazy(), start)
    }
    pub fn sequence_element_state<Curr, Next>(
        self,
        sequence: vir::ExprGenSeq<'vir, Curr, Next>,
        index: vir::ExprGenInt<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        sequence_element_state_function().call()(sequence, self.ty.lazy(), index)
    }
    pub fn state<Curr, Next>(
        self,
        snap: vir::ExprGenSnap<'vir, Curr, Next>,
        metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        state_function("indirect_resource_state").call()(
            self.caster.cast_to_callee_ctx(snap).downcast_ty(),
            self.ty.lazy(),
            metadata,
        )
    }
    pub fn apply<Curr, Next>(
        self,
        snap: vir::ExprGenSnap<'vir, Curr, Next>,
        metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenBool<'vir, Curr, Next> {
        group(
            self.caster.cast_to_callee_ctx(snap).downcast_ty(),
            self.ty.lazy(),
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
            let mut state = Some(empty_state());
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
                    state = Some(reference_state(
                        physical.ref_to_snap(address).downcast_ty(),
                        nested.state(
                            physical.ref_to_snap(address),
                            reference.generic_metadata(snap.downcast_ty()),
                        ),
                    ));
                    resources.push(nested.apply(
                        physical.ref_to_snap(address),
                        reference.generic_metadata(snap.downcast_ty()),
                    ));
                }
                TySpecifics::StructLike(data) => {
                    let mut fields = Vec::new();
                    for (field, accessor) in data.fields {
                        let resource = ResourceUse::new(deps, field.ty().decompose(task.params))?;
                        let field = accessor.read(snap.downcast_ty());
                        fields.push(resource.state(field, unit));
                        resources.push(resource.apply(field, unit));
                    }
                    state = Some(fields_state(&fields));
                }
                TySpecifics::EnumLike(data) => {
                    let discr =
                        pure.expect_enumlike().snap_to_discr_snap.call()(snap.downcast_ty());
                    for variant in data.variants {
                        let mut fields = vec![];
                        let mut field_states = vec![];
                        for (field, accessor) in variant.inner.fields {
                            field_states.push(
                                ResourceUse::new(deps, field.ty().decompose(task.params))?
                                    .state(accessor.read(snap.downcast_ty()), unit),
                            );
                            fields.push(
                                ResourceUse::new(deps, field.ty().decompose(task.params))?
                                    .apply(accessor.read(snap.downcast_ty()), unit),
                            );
                        }
                        let guard = vcx.mk_eq_expr(discr, variant.data.1.discr);
                        state = Some(vcx.mk_ternary_expr(
                            guard,
                            fields_state(&field_states),
                            state.unwrap(),
                        ));
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
                    let elements = pure.expect_array().to_seq(snap.downcast_ty(), extent);
                    // Share the canonical sequence representation with checked
                    // collection conversions and the state observer.
                    let raw = resource
                        .apply_generic(vcx.mk_seq_index_expr(elements, index).downcast_ty(), unit);
                    let indexed = IndexedResources::new(deps, raw)?;
                    let wrapped = indexed.wrap(raw, index, &[]);
                    let zero = vcx.mk_uint::<0>();
                    state = Some(fields_sequence_state(
                        resource.sequence_state(elements, zero),
                    ));
                    deps.require_ref::<ResourceSequenceEnc>(())?;
                    resources.push(vir::expr! { (zero) <= (extent) });
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
            let state = state.map(|s| s.fold_with(&mut Substitute(&substitutions)).unwrap_or(s));
            Ok((Some(ResourceCase { guard, body, state }), ()))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        emit_state_adt(program);
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
            let resource = group(vcx.mk_local_ex(s), vcx.mk_local_ex(t), vcx.mk_local_ex(m));
            let vir::ExprKindData::PredicateApp(app) = resource.kind else {
                unreachable!()
            };
            let opaque_state = state_function("opaque_resource_state");
            program.add_function(vcx.mk_function(
                opaque_state,
                (s, t, m),
                vcx.alloc_slice(&[resource]),
                &[],
                None,
                None,
            ));
            let state = cases.iter().fold(
                opaque_state.call()(vcx.mk_local_ex(s), vcx.mk_local_ex(t), vcx.mk_local_ex(m)),
                |tail, case| match case.state {
                    Some(state) => {
                        vcx.mk_ternary_expr(case.guard, vcx.mk_unfolding_expr(app, state), tail)
                    }
                    None => tail,
                },
            );
            program.add_function(vcx.mk_function(
                state_function("indirect_resource_state"),
                (s, t, m),
                vcx.alloc_slice(&[resource]),
                &[],
                None,
                Some(state),
            ));
        });
    }
}

/// Dedicated logical state: heap contents are never encoded as owned snapshots.
pub(crate) fn state_type<'vir>() -> vir::TypeCSnap<'vir> {
    vir::DomainIdnSnap::new(vir::ViperIdent::new("s_ResourceState"), 0)().downcast_ty()
}
fn state_function<'vir>(
    name: &'vir str,
) -> vir::FunctionIdn<'vir, (vir::PSnap, vir::TyVal, vir::PSnap), vir::CSnap> {
    vir::FunctionIdn::new(
        vir::ViperIdent::new(name),
        (vir::TYPE_PSNAP, vir::TYPE_TYVAL, vir::TYPE_PSNAP),
        state_type(),
    )
}
pub(crate) fn empty_state<'vir, Curr, Next>() -> vir::ExprGenCSnap<'vir, Curr, Next> {
    vir::FunctionIdn::<(), vir::CSnap>::new(vir::ViperIdent::new("ResourceEmpty"), (), state_type())
        .call()()
}
pub(crate) fn reference_state<'vir, Curr, Next>(
    stored: vir::ExprGenPSnap<'vir, Curr, Next>,
    nested: vir::ExprGenCSnap<'vir, Curr, Next>,
) -> vir::ExprGenCSnap<'vir, Curr, Next> {
    vir::FunctionIdn::<(vir::PSnap, vir::CSnap), vir::CSnap>::new(
        vir::ViperIdent::new("ResourceReference"),
        (vir::TYPE_PSNAP, state_type()),
        state_type(),
    )
    .call()(stored, nested)
}
pub(crate) fn fields_state<'vir, Curr, Next>(
    fields: &[vir::ExprGenCSnap<'vir, Curr, Next>],
) -> vir::ExprGenCSnap<'vir, Curr, Next> {
    vir::with_vcx(|vcx| {
        let seq = vcx.mk_seq_literal_expr(vcx.alloc_slice(fields), state_type());
        vir::FunctionIdn::<vir::Seq, vir::CSnap>::new(
            vir::ViperIdent::new("ResourceFields"),
            vcx.mk_ty_seq(state_type()),
            state_type(),
        )
        .call()(seq)
    })
}
fn emit_state_adt<'vir>(program: &mut task_encoder::Program<'vir>) {
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
        let fields = vcx.mk_adt_constructor(
            "ResourceFields",
            vcx.alloc_slice(&[vcx
                .mk_local_decl("resource_fields", vcx.mk_ty_seq(state_type()))
                .as_dyn()]),
        );
        let opaque = vcx.mk_adt_constructor(
            "ResourceUnknown",
            vcx.alloc_slice(&[vcx
                .mk_local_decl("resource_unknown", vir::TYPE_INT)
                .as_dyn()]),
        );
        program.add_adt(vcx.mk_adt(
            vir::ViperIdent::new("s_ResourceState"),
            &[],
            vcx.alloc_slice(&[empty, reference, fields, opaque]),
        ));
    });
}

fn sequence_state_function<'vir>()
-> vir::FunctionIdn<'vir, (vir::Seq, vir::TyVal, vir::Int), vir::Seq> {
    vir::with_vcx(|vcx| {
        vir::FunctionIdn::new(
            vir::ViperIdent::new("indirect_sequence_state"),
            (
                vcx.mk_ty_seq(vir::TYPE_PSNAP),
                vir::TYPE_TYVAL,
                vir::TYPE_INT,
            ),
            vcx.mk_ty_seq(state_type()),
        )
    })
}
fn sequence_element_state_function<'vir>()
-> vir::FunctionIdn<'vir, (vir::Seq, vir::TyVal, vir::Int), vir::CSnap> {
    vir::with_vcx(|vcx| {
        vir::FunctionIdn::new(
            vir::ViperIdent::new("indirect_sequence_element_state"),
            (
                vcx.mk_ty_seq(vir::TYPE_PSNAP),
                vir::TYPE_TYVAL,
                vir::TYPE_INT,
            ),
            state_type(),
        )
    })
}
pub(crate) fn fields_sequence_state<'vir, Curr, Next>(
    sequence: vir::ExprGenSeq<'vir, Curr, Next>,
) -> vir::ExprGenCSnap<'vir, Curr, Next> {
    vir::with_vcx(|vcx| {
        vir::FunctionIdn::<vir::Seq, vir::CSnap>::new(
            vir::ViperIdent::new("ResourceFields"),
            vcx.mk_ty_seq(state_type()),
            state_type(),
        )
        .call()(sequence)
    })
}
/// The sequence observer is needed by both collection owners and observations
/// of saved sequences. Its declaration must not depend on an array type being
/// present elsewhere in the program.
pub(crate) struct ResourceSequenceEnc;

impl TaskEncoder for ResourceSequenceEnc {
    task_encoder::encoder_cache!(ResourceSequenceEnc);
    const ENCODER_NAME: &'static str = "resource sequence observer encoder";
    type TaskDescription<'vir> = ();
    type OutputFullLocal<'vir> = (IndexedResources<'vir>, vir::ExprPSnap<'vir>);

    fn task_to_key<'vir>(_task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {}

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let unit = unit_metadata(deps)?;
        let resource = vir::with_vcx(|vcx| {
            let (snapshot, ty, _) = declarations();
            group(vcx.mk_local_ex(snapshot), vcx.mk_local_ex(ty), unit)
        });
        let indexed = IndexedResources::new(deps, resource)?;
        Ok(((indexed, unit), ()))
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for (indexed, unit) in Self::all_outputs_local_no_errors(program) {
            emit_sequence_state(program, &indexed, unit);
        }
    }
}

fn emit_sequence_state<'vir>(
    program: &mut task_encoder::Program<'vir>,
    indexed: &IndexedResources<'vir>,
    unit: vir::ExprPSnap<'vir>,
) {
    vir::with_vcx(|vcx| {
        let sequence = vcx.mk_local_decl("state_sequence", vcx.mk_ty_seq(vir::TYPE_PSNAP));
        let ty = vcx.mk_local_decl("state_element_type", vir::TYPE_TYVAL);
        let start = vcx.mk_local_decl("state_start", vir::TYPE_INT);
        let (seq, t, i) = (
            vcx.mk_local_ex(sequence),
            vcx.mk_local_ex(ty),
            vcx.mk_local_ex(start),
        );
        let length = vcx.mk_collection_len_expr(seq);
        let zero = vcx.mk_uint::<0>();
        let j_decl = vcx.mk_local_decl("state_index", vir::TYPE_INT);
        let j = vcx.mk_local_ex(j_decl);
        let element = |index| {
            vcx.mk_seq_index_expr(seq, index)
                .downcast_ty::<vir::PSnap>()
        };
        let pre = quantify_resources(
            vcx,
            vcx.alloc_slice(&[j_decl.as_dyn()]),
            &[],
            vir::expr! { ((i) <= (j)) && ((j) < (length)) },
            indexed.wrap(group(element(j), t, unit), j, &[]),
        );
        let vir::ExprKindData::PredicateApp(app) = group(element(i), t, unit).kind else {
            unreachable!()
        };
        let head = indexed
            .read_snapshot(
                app,
                state_function("indirect_resource_state").call()(element(i), t, unit).upcast_ty(),
                i,
                false,
                &[],
            )
            .downcast_ty::<vir::CSnap>();
        // Generic contracts and concrete pack/unpack helpers use one observer
        // symbol, so their quantified frames share the same heap footprint.
        program.add_function(vcx.mk_function(
            sequence_element_state_function(),
            (sequence, ty, start),
            vcx.alloc_slice(&[
                vir::expr! { ((zero) <= (i)) && ((i) < (length)) },
                indexed.wrap(group(element(i), t, unit), i, &[]),
            ]),
            &[],
            None,
            Some(head),
        ));
        let next = (vir::expr! { (i) + ([vcx.mk_uint::<1>()]) }).downcast_ty();
        let recursive = sequence_state_function().call()(seq, t, next);
        let body = vcx.mk_ternary_expr(
            vcx.mk_eq_expr(i, length),
            vcx.mk_seq_literal_expr(&[], state_type()),
            vcx.mk_seq_concat_expr(
                vcx.mk_seq_literal_expr(vcx.alloc_slice(&[head]), state_type()),
                recursive,
            ),
        );
        program.add_function(
            vcx.opaque_function(
                vcx.mk_function(
                    sequence_state_function(),
                    (sequence, ty, start),
                    vcx.alloc_slice(&[vir::expr! { ((zero) <= (i)) && ((i) <= (length)) }, pre]),
                    vcx.alloc_slice(&[vcx.mk_eq_expr(
                        vcx.mk_collection_len_expr(vcx.mk_result(vcx.mk_ty_seq(state_type()))),
                        vcx.mk_bin_op_expr(vir::BinOpKind::Sub, length, i)
                            .downcast_ty::<vir::Int>(),
                    )]),
                    None,
                    Some(body),
                ),
            ),
        );
    });
}
