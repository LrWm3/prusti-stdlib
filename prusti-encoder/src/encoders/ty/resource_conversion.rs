//! Checked conversions at polymorphic resource boundaries.
//! All operations are folds/unfolds of the actual predicate bodies.
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

use super::{
    RustTyDecomposition, RustTyNormalized, TySpecifics,
    array_typing::{ArrayTyping, ArrayTypingEnc},
    generics::{GArgCaster, GArgsCastEnc, GArgsTyEnc},
    indexed::IndexedResources,
    resource_array::{ResourceArray, ResourceArrayEnc},
    resource_collection::{ResourceCollection, ResourceCollectionEnc, ResourceCollectionTask},
    resources::{ResourceUse, unit_metadata},
    use_impure::{TyUseImpure, TyUseImpureEnc},
    use_pure::{TyUsePure, TyUsePureArray, TyUsePureEnc, TyUsePureField, TyUsePureMutRef},
};

pub(crate) struct ResourceConversionEnc;

#[derive(Clone)]
pub(crate) struct ResourceConversion<'vir> {
    pub physical: TyUseImpure<'vir>,
    resource: ResourceUse<'vir>,
    unit: vir::ExprPSnap<'vir>,
    kind: ConversionKind<'vir>,
    trivial_resources: bool,
    indexed: IndexedResources<'vir>,
    owned_slice: Option<super::owned_slice::OwnedSlice<'vir>>,
    owned_typing: Option<super::owned_slice::OwnedSliceTyping<'vir>>,
}

type Fields<'vir> = Vec<(TyUsePureField<'vir>, &'vir ResourceConversion<'vir>)>;

#[derive(Clone)]
enum ConversionKind<'vir> {
    Opaque,
    Empty,
    Array {
        array: TyUsePureArray<'vir>,
        extent: Option<vir::ExprInt<'vir>>,
        usize_ty: TyUsePure<'vir>,
        metadata: GArgCaster<'vir, crate::encoders::Pure>,
        conversion: ResourceCollection<'vir>,
        typing: Option<ArrayTyping<'vir>>,
        boundary: ResourceArray<'vir>,
    },
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
    /// A deliberately conservative test for an empty instantiated group.
    /// Finite owned fields can establish emptiness recursively. Unknown types,
    /// arrays, and raw-pointer-backed ownership cannot establish it merely
    /// through an absent concrete lifetime projection.
    pub fn has_trivial_resources(&self) -> bool {
        self.trivial_resources
    }

    pub fn supports_indexed_resources(&self) -> bool {
        match &self.kind {
            ConversionKind::Array { .. } => false,
            ConversionKind::Struct(fields) => fields
                .iter()
                .all(|(_, child)| child.supports_indexed_resources()),
            ConversionKind::Enum { variants, .. } => variants.iter().all(|(_, fields)| {
                fields
                    .iter()
                    .all(|(_, child)| child.supports_indexed_resources())
            }),
            ConversionKind::Mutable { nested, .. } => nested.supports_indexed_resources(),
            _ => true,
        }
    }

    pub fn observe<Curr, Next>(
        &self,
        snapshot: vir::ExprGenSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        self.observe_at(snapshot, self.unit.lazy(), None, &[])
    }

    pub fn observe_indexed<Curr, Next>(
        &self,
        snapshot: vir::ExprGenSnap<'vir, Curr, Next>,
        index: vir::ExprGenInt<'vir, Curr, Next>,
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        self.observe_at(snapshot, self.unit.lazy(), Some(index), &[])
    }

    fn observe_at<Curr, Next>(
        &self,
        snapshot: vir::ExprGenSnap<'vir, Curr, Next>,
        metadata: vir::ExprGenPSnap<'vir, Curr, Next>,
        index: Option<vir::ExprGenInt<'vir, Curr, Next>>,
        path: &'vir [usize],
    ) -> vir::ExprGenCSnap<'vir, Curr, Next> {
        use super::resources::{empty_state, fields_state, reference_state};
        vir::with_vcx(|vcx| {
            let fields = |fields: &Fields<'vir>| {
                fields_state(
                    &fields
                        .iter()
                        .enumerate()
                        .map(|(i, (field, nested))| {
                            nested.observe_at(
                                field.read(snapshot.downcast_ty()),
                                self.owned_slice
                                    .as_ref()
                                    .filter(|slice| slice.index == i)
                                    .map_or(self.unit.lazy(), |slice| slice.metadata(snapshot)),
                                index,
                                child_path(path, i),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            };
            match &self.kind {
                ConversionKind::Array {
                    array,
                    extent,
                    usize_ty,
                    metadata: caster,
                    conversion,
                    ..
                } => {
                    let length = extent.map(|e| e.lazy()).unwrap_or_else(|| {
                        usize_ty
                            .expect_primitive()
                            .snap_to_prim(
                                caster
                                    .cast_to_caller_ctx(metadata.upcast_ty())
                                    .downcast_ty(),
                            )
                            .downcast_ty()
                    });
                    conversion.observe(array.to_seq(snapshot.downcast_ty(), length))
                }
                ConversionKind::Opaque => {
                    let state = self.resource.state(snapshot, metadata);
                    if let Some(index) = index {
                        let vir::ExprKindGenData::PredicateApp(app) =
                            self.resource.apply(snapshot, metadata).kind
                        else {
                            unreachable!()
                        };
                        self.indexed
                            .read_snapshot(app, state.upcast_ty(), index, false, path)
                            .downcast_ty()
                    } else {
                        state
                    }
                }
                ConversionKind::Empty => empty_state(),
                ConversionKind::Mutable {
                    reference,
                    physical,
                    nested,
                } => {
                    let address = reference.deref_access(snapshot.downcast_ty());
                    let stored = physical.ref_to_snap(address);
                    let stored = index
                        .map_or(stored, |index| {
                            self.indexed.read_snapshot(
                                physical.ref_to_pred_app(address, None),
                                stored,
                                index,
                                false,
                                path,
                            )
                        })
                        .downcast_ty();
                    reference_state(
                        stored,
                        nested.observe_at(
                            reference.cast_to_caller_ctx(stored),
                            reference.generic_metadata(snapshot.downcast_ty()),
                            index,
                            child_path(path, 0),
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

    pub fn grouped(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> vir::ExprBool<'vir> {
        self.indexed
            .wrap(self.resource.apply(snapshot, self.unit), index, &[])
    }

    pub fn grouped_generic(
        &self,
        snapshot: vir::ExprPSnap<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> vir::ExprBool<'vir> {
        self.indexed
            .wrap(self.resource.apply_generic(snapshot, self.unit), index, &[])
    }

    pub fn grouped_generic_state(
        &self,
        snapshot: vir::ExprPSnap<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> vir::ExprCSnap<'vir> {
        let vir::ExprKindData::PredicateApp(app) =
            self.resource.apply_generic(snapshot, self.unit).kind
        else {
            unreachable!()
        };
        self.indexed
            .read_snapshot(
                app,
                self.resource.state_generic(snapshot, self.unit).upcast_ty(),
                index,
                false,
                &[],
            )
            .downcast_ty()
    }

    /// Concrete resources in the same structural paths used by indexed PCG expansions.
    pub fn indexed_resources(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> vir::ExprBool<'vir> {
        self.resources_at(snapshot, self.unit, index, &[])
    }

    fn resources_at(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        metadata: vir::ExprPSnap<'vir>,
        index: vir::ExprInt<'vir>,
        path: &'vir [usize],
    ) -> vir::ExprBool<'vir> {
        vir::with_vcx(|vcx| {
            let fields = |fields: &Fields<'vir>| {
                vcx.mk_conj(
                    &fields
                        .iter()
                        .enumerate()
                        .map(|(i, (field, nested))| {
                            nested.resources_at(
                                field.read(snapshot.downcast_ty()),
                                self.owned_slice
                                    .as_ref()
                                    .filter(|slice| slice.index == i)
                                    .map_or(self.unit, |slice| slice.metadata(snapshot)),
                                index,
                                child_path(path, i),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            };
            match &self.kind {
                ConversionKind::Array { .. } => unreachable!(
                    "nested collection resources were rejected by the collection encoder"
                ),
                ConversionKind::Opaque => {
                    self.indexed
                        .wrap(self.resource.apply(snapshot, metadata), index, path)
                }
                ConversionKind::Empty => vcx.mk_bool::<true>(),
                ConversionKind::Mutable {
                    reference,
                    physical,
                    nested,
                } => {
                    let address = reference.deref_access(snapshot.downcast_ty());
                    let raw = physical.ref_to_pred(vcx, address, None);
                    let stored = self.indexed.read_snapshot(
                        physical.ref_to_pred_app(address, None),
                        physical.ref_to_snap(address),
                        index,
                        false,
                        path,
                    );
                    vcx.mk_conj(&[
                        self.indexed.wrap(raw, index, path),
                        nested.resources_at(
                            reference.cast_to_caller_ctx(stored.downcast_ty()),
                            reference.generic_metadata(snapshot.downcast_ty()),
                            index,
                            child_path(path, 0),
                        ),
                    ])
                }
                ConversionKind::Struct(data) => fields(data),
                ConversionKind::Enum {
                    discriminant,
                    variants,
                } => vcx.mk_conj(
                    &variants
                        .iter()
                        .map(|(discr, data)| {
                            let guard =
                                vcx.mk_eq_expr(discriminant.call()(snapshot.downcast_ty()), *discr);
                            let body = fields(data);
                            vir::expr! { (guard) ==> (body) }
                        })
                        .collect::<Vec<_>>(),
                ),
            }
        })
    }

    pub fn at_storage_with_metadata(
        &self,
        storage: vir::ExprRef<'vir>,
        metadata: vir::ExprPSnap<'vir>,
        pack: bool,
    ) -> Vec<vir::Stmt<'vir>> {
        let mut statements = Vec::new();
        if pack && let Some(typing) = &self.owned_typing {
            statements.extend(typing.establish(storage));
        }
        if pack
            && let ConversionKind::Array {
                typing: Some(typing),
                ..
            } = &self.kind
        {
            statements.push(typing.establish(storage));
        }
        statements.extend(self.statements(self.physical.ref_to_snap(storage), metadata, pack));
        statements
    }

    pub fn statements(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        metadata: vir::ExprPSnap<'vir>,
        pack: bool,
    ) -> Vec<vir::Stmt<'vir>> {
        self.statements_at(snapshot, metadata, pack, None, &[])
    }

    pub fn indexed_statements(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        index: vir::ExprInt<'vir>,
        pack: bool,
    ) -> Vec<vir::Stmt<'vir>> {
        vir::with_vcx(|vcx| {
            let vir::ExprKindData::PredicateApp(group) = self.grouped(snapshot, index).kind else {
                unreachable!()
            };
            let mut result = Vec::new();
            if !pack {
                result.push(vcx.mk_unfold_stmt(group));
            }
            result.extend(self.statements_at(snapshot, self.unit, pack, Some(index), &[]));
            if pack {
                result.push(vcx.mk_fold_stmt(group));
            }
            result
        })
    }

    fn statements_at(
        &self,
        snapshot: vir::ExprSnap<'vir>,
        metadata: vir::ExprPSnap<'vir>,
        pack: bool,
        index: Option<vir::ExprInt<'vir>>,
        path: &'vir [usize],
    ) -> Vec<vir::Stmt<'vir>> {
        vir::with_vcx(|vcx| {
            if let ConversionKind::Array { boundary, .. } = &self.kind {
                assert!(
                    index.is_none(),
                    "nested indexed collections are unsupported"
                );
                return vec![boundary.statement(snapshot.downcast_ty(), metadata, pack)];
            }
            let resource = self.resource.apply(snapshot, metadata);
            let vir::ExprKindData::PredicateApp(app) = resource.kind else {
                unreachable!()
            };
            if matches!(self.kind, ConversionKind::Opaque) {
                return index.map_or_else(Vec::new, |index| {
                    let vir::ExprKindData::PredicateApp(app) =
                        self.indexed.wrap(resource, index, path).kind
                    else {
                        unreachable!()
                    };
                    vec![if pack {
                        vcx.mk_unfold_stmt(app)
                    } else {
                        vcx.mk_fold_stmt(app)
                    }]
                });
            }
            let mut statements = Vec::new();
            if !pack {
                statements.push(vcx.mk_unfold_stmt(app));
            }
            let fields = |fields: &Fields<'vir>| {
                fields
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (field, nested))| {
                        nested.statements_at(
                            field.read(snapshot.downcast_ty()),
                            self.owned_slice
                                .as_ref()
                                .filter(|slice| slice.index == i)
                                .map_or(self.unit, |slice| slice.metadata(snapshot)),
                            pack,
                            index,
                            child_path(path, i),
                        )
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
                    let wrapped = index.map(|index| {
                        let vir::ExprKindData::PredicateApp(app) = self
                            .indexed
                            .wrap(
                                vcx.mk_predicate_app_expr(physical.ref_to_pred_app(address, None)),
                                index,
                                path,
                            )
                            .kind
                        else {
                            unreachable!()
                        };
                        app
                    });
                    if let Some(wrapped) = wrapped.filter(|_| pack) {
                        statements.push(vcx.mk_unfold_stmt(wrapped));
                    }
                    statements.push(vcx.mk_assert_stmt(physical.ref_to_pred(vcx, address, None)));
                    let current =
                        reference.cast_to_caller_ctx(physical.ref_to_snap(address).downcast_ty());
                    statements.extend(nested.statements_at(
                        current,
                        reference.generic_metadata(snapshot.downcast_ty()),
                        pack,
                        index,
                        child_path(path, 0),
                    ));
                    if let Some(wrapped) = wrapped.filter(|_| !pack) {
                        statements.push(vcx.mk_fold_stmt(wrapped));
                    }
                }
                ConversionKind::Array { .. } => unreachable!(
                    "whole-array conversions are dispatched through their checked boundary"
                ),
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

fn child_path<'vir>(path: &[usize], child: usize) -> &'vir [usize] {
    vir::with_vcx(|vcx| vcx.alloc_slice(&path.iter().copied().chain([child]).collect::<Vec<_>>()))
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
            TySpecifics::ArrayLike(data) => {
                let element = data.0.decompose_normalize(task.args);
                let conversion = deps.require_dep::<ResourceCollectionEnc>(
                    ResourceCollectionTask::aggregate(element),
                )?;
                let usize_decomp =
                    vir::with_vcx(|vcx| RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize));
                let usize_ty = deps.require_dep::<TyUsePureEnc>(usize_decomp)?;
                let metadata = deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(Some(
                    RustTyNormalized {
                        param: RustTyDecomposition::param(),
                        concrete: usize_decomp,
                    },
                ))?;
                let extent = if data.slice {
                    None
                } else {
                    Some(
                        usize_ty
                            .expect_primitive()
                            .snap_to_prim(deps.require_dep::<GArgsTyEnc>(task.args)?.get_const()[0])
                            .downcast_ty(),
                    )
                };
                ConversionKind::Array {
                    array: pure.expect_array().clone(),
                    extent,
                    usize_ty,
                    metadata,
                    conversion,
                    typing: if data.slice {
                        None
                    } else {
                        Some(deps.require_dep::<ArrayTypingEnc>(*task)?)
                    },
                    boundary: deps.require_dep::<ResourceArrayEnc>(*task)?,
                }
            }
        };
        let trivial_resources = match &kind {
            // A raw pointer does not itself grant referent permission, but
            // using that fact to classify its enclosing owner would mistake
            // Vec/RawBuf/PhantomData representations for empty ownership.
            ConversionKind::Empty => !matches!(task.ty.specifics, TySpecifics::Raw(_)),
            ConversionKind::Struct(fields) => fields
                .iter()
                .all(|(_, child)| child.has_trivial_resources()),
            ConversionKind::Enum { variants, .. } => variants.iter().all(|(_, fields)| {
                fields
                    .iter()
                    .all(|(_, child)| child.has_trivial_resources())
            }),
            _ => false,
        };
        let indexed = vir::with_vcx(|vcx| {
            let snap =
                vcx.mk_local_ex(vcx.mk_local_decl("conversion_snapshot", physical.snapshot()));
            let mut resources = vec![resource.apply(snap, unit)];
            if let ConversionKind::Mutable { physical, .. } = &kind {
                resources.push(physical.ref_to_pred(
                    vcx,
                    vcx.mk_local_ex(vcx.mk_local_decl("conversion_address", vir::TYPE_REF)),
                    None,
                ));
            }
            IndexedResources::new(deps, vcx.mk_conj(&resources))
        })?;
        Ok((
            (),
            ResourceConversion {
                physical,
                resource,
                unit,
                kind,
                trivial_resources,
                indexed,
                owned_slice: super::owned_slice::OwnedSlice::new(deps, *task)?,
                owned_typing: super::owned_slice::OwnedSliceTyping::new(deps, *task)?,
            },
        ))
    }
}
