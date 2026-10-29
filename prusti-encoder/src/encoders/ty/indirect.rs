use itertools::Itertools;
use pcg::borrow_pcg::region_projection::{
    ExtractRegionsCtxt, LifetimeProjection, LifetimeProjectionIdx, PcgRegion, Region,
};
use prusti_rustc_interface::{
    index::IndexVec,
    middle::ty::{self, TyCtxt},
};
use std::cell::RefCell;
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::{CastType, Reify};

use crate::encoders::{TyUseImpureEnc, ty::RustTyDecomposition};

use super::{
    data::{StructData, TySpecifics},
    generics::GArgsTyEnc,
    indexed::{IndexedResources, quantify_resources},
    rust_ty::RustTyDatas,
    use_pure::{TyUsePureEnc, UsePureTyDatas},
};

#[derive(Copy, Clone)]
pub struct PrustiPcgCtxt;

impl<'tcx> ExtractRegionsCtxt<'tcx, RustTyDecomposition<'tcx>, PcgRegion<'tcx>> for PrustiPcgCtxt {
    fn extract_regions(
        self,
        data: RustTyDecomposition<'tcx>,
    ) -> IndexVec<LifetimeProjectionIdx<Region>, PcgRegion<'tcx>> {
        vir::with_vcx(|vcx| {
            data.args
                .args()
                .iter()
                .flat_map(|arg| {
                    if let Some(region) = arg.as_region() {
                        vec![PcgRegion::from(region)]
                    } else if let Some(ty) = arg.as_type() {
                        let regions: IndexVec<_, PcgRegion<'tcx>> = vcx.tcx().extract_regions(ty);
                        regions.into_iter().collect()
                    } else {
                        Vec::new()
                    }
                })
                .unique()
                .collect()
        })
    }
}

pub struct IndirectPredicatesEnc;

/// Heap reads that must be captured outside a wand when its resources refer
/// to a mutable argument's post-call state.
pub(crate) type SnapshotBindings<'vir> =
    RefCell<Vec<(vir::LocalDeclSnap<'vir>, vir::ExprSnap<'vir>)>>;
#[derive(Clone, Copy)]
pub(crate) struct ExprInput<'vir> {
    pub snap: vir::ExprSnap<'vir>,
    pub heap_label: Option<vir::OldLabel<'vir>>,
    pub bindings: Option<&'vir SnapshotBindings<'vir>>,
    pub guard: Option<vir::ExprBool<'vir>>,
    /// A slice's extent is supplied by its enclosing pointer metadata.
    pub extent: Option<vir::ExprInt<'vir>>,
    pub metadata: Option<vir::ExprPSnap<'vir>>,
    pub binder_depth: usize,
    pub resource_path: &'vir [usize],
    pub indexed: Option<(&'vir IndexedResources<'vir>, vir::ExprInt<'vir>, bool)>,
}

impl<'vir> ExprInput<'vir> {
    pub fn current(snap: vir::ExprSnap<'vir>) -> Self {
        Self {
            snap,
            heap_label: None,
            bindings: None,
            guard: None,
            extent: None,
            metadata: None,
            binder_depth: 0,
            indexed: None,
            resource_path: &[],
        }
    }
}
type ExprOutput<'vir> = vir::ExprGenBool<'vir, ExprInput<'vir>, vir::ExprKind<'vir>>;

#[derive(Clone)]
pub struct IndirectPredicatesEncOutputRef<'vir> {
    pub predicate_applications: Vec<ExprOutput<'vir>>,
    pub(crate) array: Option<ArrayIndirect<'vir>>,
}

#[derive(Clone)]
pub(crate) struct ArrayIndirect<'vir> {
    pub indexed: IndexedResources<'vir>,
}

impl<'vir> IndirectPredicatesEncOutputRef<'vir> {
    pub fn new(predicate_applications: Vec<ExprOutput<'vir>>) -> Self {
        Self {
            predicate_applications,
            array: None,
        }
    }
}

impl<'vir> task_encoder::OutputRefAny for IndirectPredicatesEncOutputRef<'vir> {}

fn projection_region<'tcx>(
    proj: &LifetimeProjection<'tcx, RustTyDecomposition<'tcx>, Region>,
) -> PcgRegion<'tcx> {
    let regions = PrustiPcgCtxt.extract_regions(proj.base());
    regions[proj.region_idx()]
}

impl TaskEncoder for IndirectPredicatesEnc {
    task_encoder::encoder_cache!(IndirectPredicatesEnc);
    const ENCODER_NAME: &'static str = "indirect predicates encoder";

    type TaskDescription<'vir> = LifetimeProjection<'vir, RustTyDecomposition<'vir>, Region>;

    type TaskKey<'tcx> = Self::TaskDescription<'tcx>;

    type EncodingError = ();

    type OutputFullDependency<'vir> = IndirectPredicatesEncOutputRef<'vir>;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        *task
    }

    fn do_encode_full<'vir>(
        task_key: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task_key, ())?;
        vir::with_vcx(|vcx| {
            let task_region = projection_region(task_key);
            let ty = task_key.base();
            let self_ty_enc = deps.require_dep::<TyUsePureEnc>(ty)?;
            let combined = ty.ty.zip(self_ty_enc);
            let mut predicate_applications = vec![];
            let mut array_indirect = None;
            // Collects (accessor, indirect_predicate) pairs for the fields of a
            // struct-like (used for structs and enum variants). Fields are
            // recursed into at their concrete (normalized) type.
            let collect_field_predicates =
                |struct_data: StructData<'vir, (RustTyDatas, UsePureTyDatas)>,
                 deps: &mut TaskEncoderDependencies<'vir, IndirectPredicatesEnc>| {
                    let mut result = vec![];
                    for (field_index, (field_ty, accessor)) in
                        struct_data.fields.into_iter().enumerate()
                    {
                        let field_ty = field_ty.decompose_normalize(ty.args);
                        if let Some(new_projection) =
                            LifetimeProjection::new(field_ty, task_region, None, PrustiPcgCtxt)
                        {
                            let field_indirect =
                                deps.require_dep::<IndirectPredicatesEnc>(new_projection)?;
                            for inner_expr in field_indirect.predicate_applications {
                                result.push((field_index, accessor, inner_expr));
                            }
                        }
                    }
                    Ok(result)
                };
            match combined.specifics {
                // Optimisation: if there are no type arguments, there cannot be
                // anything behind a ref inside (except for 'static, which we
                // ignore for now). Plus it skips unsupported types if they
                // don't have lifetimes.
                _ if ty.args.args().is_empty() => (),
                TySpecifics::Primitive(_)
                | TySpecifics::ImmRef(_)
                | TySpecifics::Raw(_)
                | TySpecifics::Builtin(_) => (),
                TySpecifics::Param(_) | TySpecifics::Opaque(_) => {
                    let resource = super::resources::ResourceUse::new(deps, ty)?;
                    let unit = super::resources::unit_metadata(deps)?;
                    predicate_applications.push(vcx.mk_lazy_expr(
                        "opaque_indirect",
                        vir::TYPE_BOOL,
                        Box::new(move |_, input: ExprInput<'vir>| {
                            let raw = resource.apply(input.snap, input.metadata.unwrap_or(unit));
                            input
                                .indexed
                                .map_or(raw, |(indexed, index, _)| {
                                    indexed.wrap(raw, index, input.resource_path)
                                })
                                .kind
                        }),
                    ));
                }
                TySpecifics::ArrayLike(data) => {
                    let element_ty = data.0.decompose_normalize(ty.args);
                    if let Some(proj) =
                        LifetimeProjection::new(element_ty, task_region, None, PrustiPcgCtxt)
                    {
                        let element = deps.require_dep::<IndirectPredicatesEnc>(proj)?;
                        if !element.predicate_applications.is_empty() {
                            let element_pure = deps.require_dep::<TyUsePureEnc>(element_ty)?;
                            let snap = vcx.mk_local_ex(
                                vcx.mk_local_decl("_element_resource", element_pure.snapshot),
                            );
                            let exprs = element
                                .predicate_applications
                                .iter()
                                .map(|expr| expr.reify(vcx, ExprInput::current(snap)))
                                .collect::<Vec<_>>();
                            let indexed = IndexedResources::new(deps, vcx.mk_conj(&exprs))?;
                            let array = self_ty_enc.expect_array();
                            let usize_ty =
                                deps.require_dep::<TyUsePureEnc>(RustTyDecomposition::from_ty(
                                    vcx.tcx().types.usize,
                                    ty.args.context(),
                                ))?;
                            let fixed_extent = if data.slice {
                                None
                            } else {
                                let args = deps.require_dep::<GArgsTyEnc>(ty.args)?;
                                Some(
                                    usize_ty
                                        .expect_primitive()
                                        .snap_to_prim(args.get_const()[0])
                                        .downcast_ty(),
                                )
                            };
                            array_indirect = Some(ArrayIndirect {
                                indexed: indexed.clone(),
                            });
                            predicate_applications.push(vcx.mk_lazy_expr(
                                "array_indirect",
                                vir::TYPE_BOOL,
                                Box::new(move |vcx, input: ExprInput<'vir>| {
                                    let extent = fixed_extent.or(input.extent).expect("slice resource extent");
                                    let index_decl = vcx.mk_local_decl(vir::vir_format!(vcx, "_resource_index{}", input.binder_depth), vir::TYPE_INT);
                                    let index = vcx.mk_local_ex(index_decl);
                                    let elements = array.to_seq(input.snap.downcast_ty(), extent);
                                    let snap = array.sequence_index(elements, index);
                                    let exprs = element.predicate_applications.iter().map(|expr| expr.reify(vcx, ExprInput {
                                        snap,
                                        // Index-dependent heap reads remain under this binder.
                                        bindings: None,
                                        extent: None,
            metadata: None,
                                        binder_depth: input.binder_depth + 1,
                                        indexed: Some((vcx.alloc(indexed.clone()), index, false)),
                                        resource_path: &[],
                                        ..input
                                    })).collect::<Vec<_>>();
                                    let resources = vcx.mk_conj(&exprs);
                                    let zero = vcx.mk_uint::<0>();
                                    let bounds = vir::expr! { (([zero]) <= (index)) && ((index) < (extent)) };
                                    let mut triggers = vec![vcx.mk_trigger(&[vcx.mk_seq_index_expr(elements, index)])];
                                    if fixed_extent.is_some() {
                                        // Fixed-array indexing may not otherwise create a
                                        // ground `to_seq` application. Slice wands must
                                        // instead depend only on the canonical sequence:
                                        // equal contents can come from distinct slice
                                        // snapshots, including the iterator's tail.
                                        triggers.push(vcx.mk_trigger(&[array.index_generic(input.snap.downcast_ty(), index).as_dyn()]));
                                    }
                                    quantify_resources(vcx,
                                        vcx.alloc_slice(&[index_decl.as_dyn()]),
                                        vcx.alloc_slice(&triggers),
                                        bounds, resources,
                                    ).kind
                                }),
                            ));
                        }
                    }
                }
                TySpecifics::MutRef((data, ref_domain)) => {
                    let inner_ty = data.referent.decompose_context(ty.ty.params, ty.args);
                    let inner_impure = deps.require_dep::<TyUseImpureEnc>(inner_ty)?;
                    let ref_region = PcgRegion::from(ty.args.args()[0].expect_region());
                    if ref_region == task_region {
                        predicate_applications.push(vcx.mk_lazy_expr(
                            "ref_indirect",
                            vir::TYPE_BOOL,
                            Box::new(move |vcx, input: ExprInput<'vir>| {
                                let addr = ref_domain.deref_access(input.snap.downcast_ty());
                                let resource = inner_impure.ref_to_pred(vcx, addr, None);
                                input
                                    .indexed
                                    .map_or(resource, |(indexed, index, _)| {
                                        indexed.wrap(resource, index, input.resource_path)
                                    })
                                    .kind
                            }),
                        ));
                    }
                    // Collect the indirect predicates of the referent itself
                    // (e.g. of the inner reference in `&'a mut &'b mut i32`).
                    let referent_ty = data.referent.decompose_normalize(ty.args);
                    let slice_metadata = matches!(referent_ty.ty.specifics, TySpecifics::ArrayLike(ref array) if array.slice)
                        .then(|| deps.require_dep::<TyUsePureEnc>(RustTyDecomposition::from_ty(vcx.tcx().types.usize, ty.args.context())))
                        .transpose()?;
                    if let Some(new_projection) =
                        LifetimeProjection::new(referent_ty, task_region, None, PrustiPcgCtxt)
                    {
                        let inner_indirect =
                            deps.require_dep::<IndirectPredicatesEnc>(new_projection)?;
                        let concrete_inner = deps.require_dep::<TyUseImpureEnc>(referent_ty)?;
                        for inner_expr in inner_indirect.predicate_applications {
                            predicate_applications.push(vcx.mk_lazy_expr(
                                "ref_inner_indirect",
                                vir::TYPE_BOOL,
                                Box::new(move |vcx, input: ExprInput<'vir>| {
                                    let inner_snap = inner_impure.ref_to_snap(
                                        ref_domain.deref_access(input.snap.downcast_ty()),
                                    );
                                    let inner_snap =
                                        if let Some((indexed, index, inspect)) = input.indexed {
                                            let addr =
                                                ref_domain.deref_access(input.snap.downcast_ty());
                                            let wrapped = indexed.read_snapshot(
                                                inner_impure.ref_to_pred_app(addr, None),
                                                inner_snap,
                                                index,
                                                inspect,
                                                input.resource_path,
                                            );
                                            if inspect {
                                                let held = vcx.mk_predicate_perm_expr(
                                                    concrete_inner.ref_to_pred_app(addr, None),
                                                );
                                                let none = vcx.mk_no_perm();
                                                vcx.mk_ternary_expr(
                                                    vir::expr! { (none) < (held) },
                                                    ref_domain
                                                        .cast_to_callee_ctx(
                                                            concrete_inner.ref_to_snap(addr),
                                                        )
                                                        .upcast_ty(),
                                                    wrapped,
                                                )
                                            } else {
                                                wrapped
                                            }
                                        } else {
                                            inner_snap
                                        };
                                    let inner_snap =
                                        vcx.maybe_apply_label(inner_snap, input.heap_label);
                                    let inner_snap = if let Some(guard) = input.guard {
                                        let fallback = ref_domain
                                            .cast_to_callee_ctx(
                                                ref_domain.value_access(input.snap.downcast_ty()),
                                            )
                                            .upcast_ty();
                                        vcx.mk_ternary_expr(guard, inner_snap, fallback)
                                    } else {
                                        inner_snap
                                    };
                                    let inner_snap = if let Some(bindings) = input.bindings {
                                        let name = vir::vir_format!(
                                            vcx,
                                            "_wand_snapshot_{}",
                                            bindings.borrow().len()
                                        );
                                        let decl = vcx.mk_local_decl(name, inner_snap.ty());
                                        bindings.borrow_mut().push((decl, inner_snap));
                                        vcx.mk_local_ex(decl)
                                    } else {
                                        inner_snap
                                    };
                                    inner_expr
                                        .reify(
                                            vcx,
                                            ExprInput {
                                                snap: ref_domain
                                                    .cast_to_caller_ctx(inner_snap.downcast_ty()),
                                                resource_path: vcx.alloc_slice(
                                                    &input
                                                        .resource_path
                                                        .iter()
                                                        .copied()
                                                        .chain([0])
                                                        .collect::<Vec<_>>(),
                                                ),
                                                metadata: Some(
                                                    ref_domain
                                                        .generic_metadata(input.snap.downcast_ty()),
                                                ),
                                                extent: slice_metadata.as_ref().map(|usize_ty| {
                                                    usize_ty
                                                        .expect_primitive()
                                                        .snap_to_prim(
                                                            ref_domain
                                                                .metadata_access(
                                                                    input.snap.downcast_ty(),
                                                                )
                                                                .downcast_ty(),
                                                        )
                                                        .downcast_ty()
                                                }),
                                                ..input
                                            },
                                        )
                                        .kind
                                }),
                            ));
                        }
                    }
                }
                TySpecifics::StructLike(data) => {
                    // TODO: invalid recursion here if the defined struct is
                    // recursive!
                    for (field_index, accessor, inner_expr) in collect_field_predicates(data, deps)?
                    {
                        predicate_applications.push(vcx.mk_lazy_expr(
                            "struct_field_indirect",
                            vir::TYPE_BOOL,
                            Box::new(move |vcx, input: ExprInput<'vir>| {
                                inner_expr
                                    .reify(
                                        vcx,
                                        ExprInput {
                                            snap: accessor.read(input.snap.downcast_ty()),
                                            resource_path: vcx.alloc_slice(
                                                &input
                                                    .resource_path
                                                    .iter()
                                                    .copied()
                                                    .chain([field_index])
                                                    .collect::<Vec<_>>(),
                                            ),
                                            ..input
                                        },
                                    )
                                    .kind
                            }),
                        ));
                    }
                }
                TySpecifics::EnumLike(data) => {
                    let snap_to_discr_snap = data.data.1.snap_to_discr_snap;

                    let variant_preds = data
                        .variants
                        .into_iter()
                        .map(|variant| {
                            let fields = collect_field_predicates(variant.inner, deps)?;
                            Ok((variant.data.1.discr, fields))
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    // Enums such as Option<&T> have no indirect mutable
                    // resources in any variant. Keep that absence explicit,
                    // rather than emitting a discriminant-guarded `true` that
                    // makes loop encoding demand a nonexistent resource wand.
                    if variant_preds.iter().all(|(_, fields)| fields.is_empty()) {
                        return Ok(((), IndirectPredicatesEncOutputRef::new(vec![])));
                    }

                    predicate_applications.push(vcx.mk_lazy_expr(
                        "enum_variant_indirect",
                        vir::TYPE_BOOL,
                        Box::new(move |vcx, input: ExprInput<'vir>| {
                            let self_csnap = input.snap.downcast_ty();
                            let self_discr = snap_to_discr_snap.call()(self_csnap);
                            let variant_conjs: Vec<_> = variant_preds
                                .iter()
                                .enumerate()
                                .map(|(variant_index, (discr, fields))| {
                                    let is_variant = vcx.mk_eq_expr(self_discr, *discr);
                                    let inner_guard =
                                        Some(input.guard.map_or(is_variant, |guard| {
                                            vcx.mk_conj(&[guard, is_variant])
                                        }));
                                    let preds: Vec<_> = fields
                                        .iter()
                                        .map(|(field_index, acc, expr)| {
                                            expr.reify(
                                                vcx,
                                                ExprInput {
                                                    snap: acc.read(self_csnap),
                                                    guard: inner_guard,
                                                    resource_path: vcx.alloc_slice(
                                                        &input
                                                            .resource_path
                                                            .iter()
                                                            .copied()
                                                            .chain([variant_index, *field_index])
                                                            .collect::<Vec<_>>(),
                                                    ),
                                                    ..input
                                                },
                                            )
                                        })
                                        .collect();
                                    (discr, vcx.mk_conj(&preds))
                                })
                                .collect();
                            let (first, rest) = variant_conjs.split_first().unwrap();
                            rest.iter()
                                .fold(first.1, |else_, (discr, conj)| {
                                    vcx.mk_ternary_expr(
                                        vir::expr! { ([self_discr]) == ([*discr]) },
                                        *conj,
                                        else_,
                                    )
                                })
                                .kind
                        }),
                    ));
                }
            };
            Ok((
                (),
                IndirectPredicatesEncOutputRef {
                    predicate_applications,
                    array: array_indirect,
                },
            ))
        })
    }
}

pub fn projection_for_generalized_idx<'tcx>(
    ty: ty::Ty<'tcx>,
    idx: LifetimeProjectionIdx<pcg::borrow_pcg::region_projection::Generalized>,
    decomp: RustTyDecomposition<'tcx>,
    tcx: TyCtxt<'tcx>,
) -> Option<LifetimeProjection<'tcx, RustTyDecomposition<'tcx>, Region>> {
    use pcg::borrow_pcg::GeneralizedLifetime;
    let lifetimes: IndexVec<_, GeneralizedLifetime<'tcx>> = tcx.extract_regions(ty);
    let region = match lifetimes.get(idx)? {
        GeneralizedLifetime::Region(r) => *r,
        GeneralizedLifetime::RegionsIn(ty) => PcgRegion::RegionsIn(*ty),
    };
    LifetimeProjection::new(decomp, region, None, PrustiPcgCtxt)
}
