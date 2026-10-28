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
        data.args
            .args()
            .iter()
            .flat_map(|arg| arg.walk())
            .filter_map(|arg| arg.as_region().map(PcgRegion::from))
            .unique()
            .collect()
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
}

impl<'vir> ExprInput<'vir> {
    pub fn current(snap: vir::ExprSnap<'vir>) -> Self {
        Self {
            snap,
            heap_label: None,
            bindings: None,
            guard: None,
        }
    }
}
type ExprOutput<'vir> = vir::ExprGenBool<'vir, ExprInput<'vir>, vir::ExprKind<'vir>>;

#[derive(Clone)]
pub struct IndirectPredicatesEncOutputRef<'vir> {
    pub predicate_applications: Vec<ExprOutput<'vir>>,
}

impl<'vir> IndirectPredicatesEncOutputRef<'vir> {
    pub fn new(predicate_applications: Vec<ExprOutput<'vir>>) -> Self {
        Self {
            predicate_applications,
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
            // Collects (accessor, indirect_predicate) pairs for the fields of a
            // struct-like (used for structs and enum variants). Fields are
            // recursed into at their concrete (normalized) type.
            let collect_field_predicates =
                |struct_data: StructData<'vir, (RustTyDatas, UsePureTyDatas)>,
                 deps: &mut TaskEncoderDependencies<'vir, IndirectPredicatesEnc>| {
                    let mut result = vec![];
                    for (field_ty, accessor) in struct_data.fields {
                        let field_ty = field_ty.decompose_normalize(ty.args);
                        if let Some(new_projection) =
                            LifetimeProjection::new(field_ty, task_region, None, PrustiPcgCtxt)
                        {
                            let field_indirect =
                                deps.require_dep::<IndirectPredicatesEnc>(new_projection)?;
                            for inner_expr in field_indirect.predicate_applications {
                                result.push((accessor, inner_expr));
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
                // TODO: it's not valid to have nothing for these. We should fix
                // this by using an opaque predicate to represent potential
                // indirect stuff. For example:
                // fn foo<'a, T: Trait<'a>>(x: T) -> &'a mut i32 { x.get() }
                // Here, `T` could be instantiated as `&'a mut i32` in which
                // case we would want a wand with `i32(result) --* opaque_behind_a(x)`.
                // This is why we should return `opaque_behind_a(x)` here.
                TySpecifics::Param(_) | TySpecifics::Opaque(_) | TySpecifics::ArrayLike(_) => (),
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
                                inner_impure.ref_to_pred(vcx, addr, None).kind
                            }),
                        ));
                    }
                    // Collect the indirect predicates of the referent itself
                    // (e.g. of the inner reference in `&'a mut &'b mut i32`).
                    let referent_ty = data.referent.decompose_normalize(ty.args);
                    if let Some(new_projection) =
                        LifetimeProjection::new(referent_ty, task_region, None, PrustiPcgCtxt)
                    {
                        let inner_indirect =
                            deps.require_dep::<IndirectPredicatesEnc>(new_projection)?;
                        for inner_expr in inner_indirect.predicate_applications {
                            predicate_applications.push(vcx.mk_lazy_expr(
                                "ref_inner_indirect",
                                vir::TYPE_BOOL,
                                Box::new(move |vcx, input: ExprInput<'vir>| {
                                    let inner_snap = inner_impure.ref_to_snap(
                                        ref_domain.deref_access(input.snap.downcast_ty()),
                                    );
                                    let inner_snap =
                                        vcx.maybe_apply_label(inner_snap, input.heap_label);
                                    // A reference inside an inactive enum variant has no
                                    // referent permission. Use its arbitrary snapshot field
                                    // on that branch, which is ignored by the variant test.
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
                    for (accessor, inner_expr) in collect_field_predicates(data, deps)? {
                        predicate_applications.push(vcx.mk_lazy_expr(
                            "struct_field_indirect",
                            vir::TYPE_BOOL,
                            Box::new(move |vcx, input: ExprInput<'vir>| {
                                inner_expr
                                    .reify(
                                        vcx,
                                        ExprInput {
                                            snap: accessor.read(input.snap.downcast_ty()),
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

                    if variant_preds.is_empty() {
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
                                .map(|(discr, fields)| {
                                    let is_variant = vcx.mk_eq_expr(self_discr, *discr);
                                    let inner_guard =
                                        Some(input.guard.map_or(is_variant, |guard| {
                                            vcx.mk_conj(&[guard, is_variant])
                                        }));
                                    let preds: Vec<_> = fields
                                        .iter()
                                        .map(|(acc, expr)| {
                                            expr.reify(
                                                vcx,
                                                ExprInput {
                                                    snap: acc.read(self_csnap),
                                                    guard: inner_guard,
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
                IndirectPredicatesEncOutputRef::new(predicate_applications),
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
        GeneralizedLifetime::RegionsIn(_) => return None,
    };
    LifetimeProjection::new(decomp, region, None, PrustiPcgCtxt)
}
