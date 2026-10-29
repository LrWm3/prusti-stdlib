//! Checked conversions at polymorphic resource boundaries.
//! All operations are folds/unfolds of the actual predicate bodies.
use task_encoder::{EncodeFullError, EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

use super::{
    RustTyDecomposition, TySpecifics,
    resources::{ResourceUse, unit_metadata},
    use_impure::{TyUseImpure, TyUseImpureEnc},
    use_pure::{TyUsePureEnc, TyUsePureField, TyUsePureMutRef},
};

pub(crate) struct ResourceConversionEnc;

#[derive(Clone)]
pub(crate) struct ResourceConversion<'vir> {
    pub physical: TyUseImpure<'vir>,
    resource: ResourceUse<'vir>,
    unit: vir::ExprPSnap<'vir>,
    kind: ConversionKind<'vir>,
}

type Fields<'vir> = Vec<(TyUsePureField<'vir>, &'vir ResourceConversion<'vir>)>;

#[derive(Clone)]
enum ConversionKind<'vir> {
    Opaque,
    Empty,
    Struct(Fields<'vir>),
    Enum {
        discriminant: vir::FunctionIdn<'vir, vir::CSnap, vir::CSnap>,
        variants: Vec<(vir::ExprCSnap<'vir>, Fields<'vir>)>,
    },
    Mutable {
        reference: TyUsePureMutRef<'vir>,
        physical: TyUseImpure<'vir>,
        nested: &'vir ResourceConversion<'vir>,
    },
}

impl<'vir> ResourceConversion<'vir> {
    pub fn observe<Curr, Next>(
        &self,
        snapshot: vir::ExprGenSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        self.observe_with_metadata(snapshot, self.unit.lazy())
    }

    fn observe_with_metadata<Curr, Next>(
        &self,
        snapshot: vir::ExprGenSnap<'vir, Curr, Next>,
        metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        use super::resources::{empty_state, fields_state, reference_state};
        vir::with_vcx(|vcx| {
            let fields = |fields: &Fields<'vir>| {
                fields_state(
                    &fields
                        .iter()
                        .map(|(field, nested)| nested.observe(field.read(snapshot.downcast_ty())))
                        .collect::<Vec<_>>(),
                )
            };
            match &self.kind {
                ConversionKind::Opaque => self.resource.state(snapshot, metadata),
                ConversionKind::Empty => empty_state(),
                ConversionKind::Mutable {
                    reference,
                    physical,
                    nested,
                } => {
                    let stored = physical
                        .ref_to_snap(reference.deref_access(snapshot.downcast_ty()))
                        .downcast_ty();
                    reference_state(
                        stored,
                        nested.observe_with_metadata(
                            reference.cast_to_caller_ctx(stored),
                            reference.generic_metadata(snapshot.downcast_ty()),
                        ),
                    )
                }
                ConversionKind::Struct(data) => fields(data),
                ConversionKind::Enum {
                    discriminant,
                    variants,
                } => {
                    let current = discriminant.call()(snapshot.downcast_ty());
                    variants.iter().fold(empty_state(), |tail, (discr, data)| {
                        vcx.mk_ternary_expr(
                            vcx.mk_eq_expr(current, discr.lazy()),
                            fields(data),
                            tail,
                        )
                    })
                }
            }
        })
    }

    pub fn at_snapshot(&self, snapshot: vir::ExprSnap<'vir>, pack: bool) -> Vec<vir::Stmt<'vir>> {
        self.statements(snapshot, self.unit, pack)
    }

    pub fn at_storage(&self, storage: vir::ExprRef<'vir>, pack: bool) -> Vec<vir::Stmt<'vir>> {
        self.statements(self.physical.ref_to_snap(storage), self.unit, pack)
    }

    pub fn statements(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        metadata: vir::ExprPSnap<'vir>,
        pack: bool,
    ) -> Vec<vir::Stmt<'vir>> {
        if matches!(self.kind, ConversionKind::Opaque) {
            return Vec::new();
        }
        vir::with_vcx(|vcx| {
            let resource = self.resource.apply(snapshot, metadata);
            let vir::ExprKindData::PredicateApp(app) = resource.kind else {
                unreachable!()
            };
            let mut statements = Vec::new();
            if !pack {
                statements.push(vcx.mk_unfold_stmt(app));
            }
            let fields = |fields: &Fields<'vir>| {
                fields
                    .iter()
                    .flat_map(|(field, nested)| {
                        nested.statements(field.read(snapshot.downcast_ty()), self.unit, pack)
                    })
                    .collect::<Vec<_>>()
            };
            match &self.kind {
                ConversionKind::Mutable {
                    reference,
                    physical,
                    nested,
                } => {
                    let address = reference.deref_access(snapshot.downcast_ty());
                    statements.push(vcx.mk_assert_stmt(physical.ref_to_pred(vcx, address, None)));
                    let current =
                        reference.cast_to_caller_ctx(physical.ref_to_snap(address).downcast_ty());
                    statements.extend(nested.statements(
                        current,
                        reference.generic_metadata(snapshot.downcast_ty()),
                        pack,
                    ));
                }
                ConversionKind::Struct(data) => statements.extend(fields(data)),
                ConversionKind::Enum {
                    discriminant,
                    variants,
                } => {
                    let current = discriminant.call()(snapshot.downcast_ty());
                    for (discriminant, data) in variants {
                        statements.push(vcx.mk_if_stmt(
                            vcx.mk_eq_expr(current, *discriminant),
                            vcx.alloc_slice(&fields(data)),
                            &[],
                        ));
                    }
                }
                ConversionKind::Opaque | ConversionKind::Empty => (),
            }
            if pack {
                statements.push(vcx.mk_fold_stmt(app));
            }
            statements
        })
    }
}

impl TaskEncoder for ResourceConversionEnc {
    task_encoder::encoder_cache!(ResourceConversionEnc);
    const ENCODER_NAME: &'static str = "checked resource conversion encoder";
    type TaskDescription<'vir> = RustTyDecomposition<'vir>;
    type OutputFullDependency<'vir> = ResourceConversion<'vir>;
    type EncodingError = String;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        *task
    }
    fn describe_error(error: Self::EncodingError) -> String {
        error
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let pure = deps.require_dep::<TyUsePureEnc>(*task)?;
        let physical = deps.require_dep::<TyUseImpureEnc>(*task)?;
        let resource = ResourceUse::new(deps, *task)?;
        let unit = unit_metadata(deps)?;
        let kind = match task.ty.zip(pure).specifics {
            TySpecifics::Param(_) | TySpecifics::Opaque(_) => ConversionKind::Opaque,
            TySpecifics::Primitive(_)
            | TySpecifics::ImmRef(_)
            | TySpecifics::Raw(_)
            | TySpecifics::Builtin(_) => ConversionKind::Empty,
            TySpecifics::MutRef((data, reference)) => {
                let physical = deps.require_dep::<TyUseImpureEnc>(
                    data.referent.decompose_context(task.ty.params, task.args),
                )?;
                let nested = deps.require_dep::<ResourceConversionEnc>(
                    data.referent.decompose_normalize(task.args),
                )?;
                ConversionKind::Mutable {
                    reference: reference.clone(),
                    physical,
                    nested: vir::with_vcx(|vcx| vcx.alloc(nested)),
                }
            }
            TySpecifics::StructLike(data) => {
                let mut fields = Vec::new();
                for (field, accessor) in data.fields {
                    let nested = deps.require_dep::<ResourceConversionEnc>(
                        field.ty().decompose_normalize(task.args),
                    )?;
                    fields.push((*accessor, vir::with_vcx(|vcx| &*vcx.alloc(nested))));
                }
                ConversionKind::Struct(fields)
            }
            TySpecifics::EnumLike(data) => {
                let mut variants = Vec::new();
                for variant in data.variants {
                    let mut fields = Vec::new();
                    for (field, accessor) in variant.inner.fields {
                        let nested = deps.require_dep::<ResourceConversionEnc>(
                            field.ty().decompose_normalize(task.args),
                        )?;
                        fields.push((*accessor, vir::with_vcx(|vcx| &*vcx.alloc(nested))));
                    }
                    variants.push((variant.data.1.discr, fields));
                }
                ConversionKind::Enum {
                    discriminant: pure.expect_enumlike().snap_to_discr_snap,
                    variants,
                }
            }
            TySpecifics::ArrayLike(_) => {
                return Err(EncodeFullError::EncodingError(
                    "checked collection resource conversion is not implemented yet".into(),
                    None,
                ));
            }
        };
        Ok((
            (),
            ResourceConversion {
                physical,
                resource,
                unit,
                kind,
            },
        ))
    }
}
