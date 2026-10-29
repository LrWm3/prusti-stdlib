//! Convert only the opaque groups introduced by a generic signature.
//! A constructor's physical storage stays separate from these resources.
use super::{
    RustTyDecomposition, RustTyNormalized, TySpecifics,
    array_typing::{ArrayTyping, ArrayTypingEnc},
    generics::{GArgCaster, GArgsCastEnc, GArgsTyEnc},
    resource_collection::{ResourceCollection, ResourceCollectionEnc},
    resource_conversion::{ResourceConversion, ResourceConversionEnc},
    resources::unit_metadata,
    use_impure::{TyUseImpure, TyUseImpureEnc},
    use_pure::{TyUsePureArray, TyUsePureEnc, TyUsePureField, TyUsePureMutRef},
};
use crate::encoders::{Impure, Pure};
use prusti_rustc_interface::middle::ty;
use task_encoder::{EncodeFullError, EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

pub(crate) struct ResourceBoundaryEnc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ResourceBoundaryTask<'vir> {
    pub definition: RustTyDecomposition<'vir>,
    pub actual: RustTyDecomposition<'vir>,
    /// None converts all definition-site opaque groups; Some selects one.
    pub selected: Option<ty::Ty<'vir>>,
}

type Fields<'vir> = Vec<(TyUsePureField<'vir>, ResourceBoundary<'vir>)>;

#[derive(Clone)]
pub(crate) struct ResourceBoundary<'vir> {
    pub physical: TyUseImpure<'vir>,
    unit: vir::ExprPSnap<'vir>,
    kind: BoundaryKind<'vir>,
}

#[derive(Clone)]
enum BoundaryKind<'vir> {
    Empty,
    Leaf(ResourceConversion<'vir>),
    Reference {
        reference: TyUsePureMutRef<'vir>,
        physical: TyUseImpure<'vir>,
        cast: GArgCaster<'vir, Impure>,
        nested: Box<ResourceBoundary<'vir>>,
    },
    Fields(Fields<'vir>),
    Variants {
        discriminant: vir::FunctionIdn<'vir, vir::CSnap, vir::CSnap>,
        variants: Vec<(vir::ExprCSnap<'vir>, Fields<'vir>)>,
    },
    Array {
        array: TyUsePureArray<'vir>,
        extent: Option<vir::ExprInt<'vir>>,
        metadata: GArgCaster<'vir, Pure>,
        usize_ty: super::use_pure::TyUsePure<'vir>,
        typing: ArrayTyping<'vir>,
        collection: ResourceCollection<'vir>,
    },
}

impl<'vir> ResourceBoundary<'vir> {
    pub fn is_empty(&self) -> bool {
        matches!(self.kind, BoundaryKind::Empty)
    }

    pub fn at_storage(&self, storage: vir::ExprRef<'vir>, pack: bool) -> Vec<vir::Stmt<'vir>> {
        self.storage(storage, self.unit, pack)
    }

    fn storage(
        &self,
        storage: vir::ExprRef<'vir>,
        metadata: vir::ExprPSnap<'vir>,
        pack: bool,
    ) -> Vec<vir::Stmt<'vir>> {
        if let BoundaryKind::Leaf(conversion) = &self.kind {
            return conversion.at_storage_with_metadata(storage, metadata, pack);
        }
        let mut statements = Vec::new();
        if pack && let BoundaryKind::Array { typing, .. } = &self.kind {
            statements.push(typing.establish_extent(storage, self.extent(metadata)));
        }
        statements.extend(self.snapshot(self.physical.ref_to_snap(storage), metadata, pack, None));
        statements
    }

    pub fn at_snapshot(&self, snapshot: vir::ExprSnap<'vir>, pack: bool) -> Vec<vir::Stmt<'vir>> {
        self.snapshot(snapshot, self.unit, pack, None)
    }

    pub fn at_snapshot_on_heap(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        pack: bool,
        label: &'vir str,
    ) -> Vec<vir::Stmt<'vir>> {
        self.snapshot(snapshot, self.unit, pack, Some(label))
    }

    fn extent(&self, metadata: vir::ExprPSnap<'vir>) -> vir::ExprInt<'vir> {
        let BoundaryKind::Array {
            extent,
            metadata: cast,
            usize_ty,
            ..
        } = &self.kind
        else {
            unreachable!()
        };
        extent.unwrap_or_else(|| {
            usize_ty
                .expect_primitive()
                .snap_to_prim(cast.cast_to_caller_ctx(metadata.upcast_ty()).downcast_ty())
                .downcast_ty()
        })
    }

    fn snapshot(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        metadata: vir::ExprPSnap<'vir>,
        pack: bool,
        label: Option<&'vir str>,
    ) -> Vec<vir::Stmt<'vir>> {
        vir::with_vcx(|vcx| {
            let fields = |fields: &Fields<'vir>| {
                fields
                    .iter()
                    .flat_map(|(field, nested)| {
                        nested.snapshot(field.read(snapshot.downcast_ty()), self.unit, pack, label)
                    })
                    .collect::<Vec<_>>()
            };
            match &self.kind {
                BoundaryKind::Empty => Vec::new(),
                BoundaryKind::Leaf(conversion) => conversion.statements(snapshot, metadata, pack),
                BoundaryKind::Reference {
                    reference,
                    physical,
                    cast,
                    nested,
                } => {
                    let address = reference.deref_access(snapshot.downcast_ty());
                    let metadata = reference.generic_metadata(snapshot.downcast_ty());
                    if let Some(label) = label {
                        // Match the heap used by the definition-site lifetime
                        // projection, including interior reference identities.
                        let stored =
                            vcx.mk_local_labelled_old_expr(physical.ref_to_snap(address), label);
                        let value = reference.cast_to_caller_ctx(stored.downcast_ty());
                        return nested.snapshot(value, metadata, pack, Some(label));
                    }
                    let mut statements = cast
                        .cast_to_caller_ctx(address)
                        .into_iter()
                        .collect::<Vec<_>>();
                    statements.extend(nested.storage(address, metadata, pack));
                    statements.extend(cast.cast_to_callee_ctx(address));
                    statements
                }
                BoundaryKind::Fields(data) => fields(data),
                BoundaryKind::Variants {
                    discriminant,
                    variants,
                } => variants
                    .iter()
                    .map(|(value, data)| {
                        vcx.mk_if_stmt(
                            vcx.mk_eq_expr(discriminant.call()(snapshot.downcast_ty()), *value),
                            vcx.alloc_slice(&fields(data)),
                            &[],
                        )
                    })
                    .collect(),
                BoundaryKind::Array {
                    array, collection, ..
                } => collection.statements(
                    array.to_seq(snapshot.downcast_ty(), self.extent(metadata)),
                    pack,
                ),
            }
        })
    }
}

impl TaskEncoder for ResourceBoundaryEnc {
    task_encoder::encoder_cache!(ResourceBoundaryEnc);
    const ENCODER_NAME: &'static str = "generic constructor resource boundary encoder";
    type TaskDescription<'vir> = ResourceBoundaryTask<'vir>;
    type OutputFullDependency<'vir> = ResourceBoundary<'vir>;
    type EncodingError = String;
    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        *task
    }
    fn describe_error(error: String) -> String {
        error
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let definition = task.definition;
        let actual = task.actual;
        let physical = deps.require_dep::<TyUseImpureEnc>(actual)?;
        let unit = unit_metadata(deps)?;
        let pure = deps.require_dep::<TyUsePureEnc>(actual)?;
        if definition.ty == actual.ty && definition.args.args() == actual.args.args() {
            return Ok((
                (),
                ResourceBoundary {
                    physical,
                    unit,
                    kind: BoundaryKind::Empty,
                },
            ));
        }
        let mut nested = |definition, actual| {
            deps.require_dep::<Self>(ResourceBoundaryTask {
                definition,
                actual,
                selected: task.selected,
            })
        };
        let kind = match &definition.ty.specifics {
            TySpecifics::Param(_) | TySpecifics::Opaque(_) => {
                let selected = task.selected.is_none_or(|selected| {
                    definition.args.args().first().and_then(|arg| arg.as_type()) == Some(selected)
                });
                if selected && !actual.ty.specifics.is_param() {
                    BoundaryKind::Leaf(deps.require_dep::<ResourceConversionEnc>(actual)?)
                } else {
                    BoundaryKind::Empty
                }
            }
            TySpecifics::MutRef(data) => {
                let TySpecifics::MutRef(actual_data) = &actual.ty.specifics else {
                    unreachable!()
                };
                let referent = actual_data.referent.decompose_normalize(actual.args);
                let child = nested(data.referent.decompose_normalize(definition.args), referent)?;
                if child.is_empty() {
                    BoundaryKind::Empty
                } else {
                    BoundaryKind::Reference {
                        reference: pure.expect_mutref().clone(),
                        physical: deps.require_dep::<TyUseImpureEnc>(
                            actual_data
                                .referent
                                .decompose_context(actual.ty.params, actual.args),
                        )?,
                        cast: deps.require_dep::<GArgsCastEnc<Impure>>(
                            (!referent.ty.specifics.is_param()).then_some(RustTyNormalized {
                                param: RustTyDecomposition::param(),
                                concrete: referent,
                            }),
                        )?,
                        nested: Box::new(child),
                    }
                }
            }
            TySpecifics::StructLike(data) => {
                let TySpecifics::StructLike(actual_data) = &actual.ty.specifics else {
                    unreachable!()
                };
                let mut fields = Vec::new();
                for ((field, actual_field), accessor) in data
                    .fields
                    .iter()
                    .zip(&actual_data.fields)
                    .zip(&pure.expect_structlike().fields)
                {
                    let child = nested(
                        field.ty().decompose_normalize(definition.args),
                        actual_field.ty().decompose_normalize(actual.args),
                    )?;
                    if !child.is_empty() {
                        fields.push((*accessor, child));
                    }
                }
                if fields.is_empty() {
                    BoundaryKind::Empty
                } else {
                    BoundaryKind::Fields(fields)
                }
            }
            TySpecifics::EnumLike(data) => {
                let TySpecifics::EnumLike(actual_data) = &actual.ty.specifics else {
                    unreachable!()
                };
                let mut variants = Vec::new();
                for ((variant, actual_variant), accessors) in data
                    .variants
                    .iter()
                    .zip(&actual_data.variants)
                    .zip(&pure.expect_enumlike().variants)
                {
                    let mut fields = Vec::new();
                    for ((field, actual_field), accessor) in variant
                        .inner
                        .fields
                        .iter()
                        .zip(&actual_variant.inner.fields)
                        .zip(&accessors.inner.fields)
                    {
                        let child = nested(
                            field.ty().decompose_normalize(definition.args),
                            actual_field.ty().decompose_normalize(actual.args),
                        )?;
                        if !child.is_empty() {
                            fields.push((*accessor, child));
                        }
                    }
                    if !fields.is_empty() {
                        variants.push((accessors.data.discr, fields));
                    }
                }
                if variants.is_empty() {
                    BoundaryKind::Empty
                } else {
                    BoundaryKind::Variants {
                        discriminant: pure.expect_enumlike().snap_to_discr_snap,
                        variants,
                    }
                }
            }
            TySpecifics::ArrayLike(data) => {
                let TySpecifics::ArrayLike(actual_data) = &actual.ty.specifics else {
                    unreachable!()
                };
                let element = data.decompose_normalize(definition.args);
                let actual_element = actual_data.decompose_normalize(actual.args);
                let child = nested(element, actual_element)?;
                if child.is_empty() {
                    BoundaryKind::Empty
                } else if !element.ty.specifics.is_param() {
                    return Err(EncodeFullError::EncodingError("resource conversions for generic parameters nested inside collection elements are unsupported".into(), None));
                } else {
                    let usize_decomp = vir::with_vcx(|vcx| {
                        RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize)
                    });
                    let usize_ty = deps.require_dep::<TyUsePureEnc>(usize_decomp)?;
                    let args = deps.require_dep::<GArgsTyEnc>(actual.args)?;
                    BoundaryKind::Array {
                        array: pure.expect_array().clone(),
                        extent: (!data.slice).then(|| {
                            usize_ty
                                .expect_primitive()
                                .snap_to_prim(args.get_const()[0])
                                .downcast_ty()
                        }),
                        metadata: deps.require_dep::<GArgsCastEnc<Pure>>(Some(
                            RustTyNormalized {
                                param: RustTyDecomposition::param(),
                                concrete: usize_decomp,
                            },
                        ))?,
                        usize_ty,
                        typing: deps.require_dep::<ArrayTypingEnc>(actual)?,
                        collection: deps.require_dep::<ResourceCollectionEnc>(actual_element)?,
                    }
                }
            }
            _ => BoundaryKind::Empty,
        };
        Ok((
            (),
            ResourceBoundary {
                physical,
                unit,
                kind,
            },
        ))
    }
}
