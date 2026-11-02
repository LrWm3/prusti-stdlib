//! Checked separation of indexed scalar references from a standalone scalar.
//! Abstract predicates alone do not imply disjoint addresses. These lemmas
//! expose actual primitive fields and preserve both resources and observations.
use super::{
    RustTyDecomposition, RustTyNormalized, TySpecifics,
    generics::{GArgsCastEnc, GenericParams, GenericParamsEnc, TyExprEnc},
    indexed::IndexedResources,
    indirect::{ExprInput, IndirectPredicatesEnc},
    use_impure::TyUseImpureEnc,
};
use pcg::borrow_pcg::region_projection::LifetimeProjection;
use std::hash::{Hash, Hasher};
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::{CastType, Foldable, Folder, MethodIdn, Reify};

type Arguments = (vir::Seq, vir::Int, vir::Ref, vir::ManyTyVal, vir::ManyCSnap);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexedSeparationTask<'vir> {
    pub projection: LifetimeProjection<'vir, RustTyDecomposition<'vir>>,
    pub scalar: RustTyDecomposition<'vir>,
}
pub(crate) struct IndexedSeparationEnc;
#[derive(Clone)]
pub(crate) struct IndexedSeparation<'vir> {
    methods: Vec<MethodIdn<'vir, Arguments>>,
    params: GenericParams<'vir>,
}
impl<'vir> IndexedSeparation<'vir> {
    pub fn statements(
        &self,
        sequence: vir::ExprSeq<'vir>,
        address: vir::ExprRef<'vir>,
    ) -> Vec<vir::Stmt<'vir>> {
        vir::with_vcx(|vcx| {
            self.methods
                .iter()
                .map(|method| {
                    method(
                        sequence,
                        vcx.mk_uint::<0>(),
                        address,
                        self.params.ty_exprs(),
                        self.params.const_exprs(),
                    )
                    .alloc()
                })
                .collect()
        })
    }
}

fn stable(expr: vir::ExprDyn<'_>) -> bool {
    struct Check(bool);
    impl<'vir> Folder<'vir, (), !> for Check {
        fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
            match expr.kind {
                vir::ExprKindData::Local(_)
                | vir::ExprKindData::Const(_)
                | vir::ExprKindData::AdtDestructor(..)
                | vir::ExprKindData::BinOp(_)
                | vir::ExprKindData::UnOp(_)
                | vir::ExprKindData::CollectionBinOp(_)
                | vir::ExprKindData::CollectionLen(_) => {}
                vir::ExprKindData::FuncApp(app)
                    if app.target.starts_with("s_")
                        || app.target.starts_with("make_generic_s_") => {}
                _ => {
                    self.0 = false;
                    return None;
                }
            }
            expr.super_fold_with(self)
        }
    }
    let mut check = Check(true);
    let _ = expr.fold_with(&mut check);
    check.0
}

fn leaves<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    expr: vir::ExprBool<'vir>,
    guard: vir::ExprBool<'vir>,
    out: &mut Vec<(vir::ExprBool<'vir>, vir::PredicateApp<'vir>)>,
) {
    match expr.kind {
        vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => {
            leaves(vcx, op.lhs.downcast_ty(), guard, out);
            leaves(vcx, op.rhs.downcast_ty(), guard, out);
        }
        vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::Implies && stable(op.lhs) => {
            leaves(
                vcx,
                op.rhs.downcast_ty(),
                vcx.mk_conj(&[guard, op.lhs.downcast_ty()]),
                out,
            )
        }
        vir::ExprKindData::Ternary(branch) if stable(branch.cond.as_dyn()) => {
            leaves(
                vcx,
                branch.then.downcast_ty(),
                vcx.mk_conj(&[guard, branch.cond]),
                out,
            );
            let otherwise = vcx
                .mk_unary_op_expr(vir::UnOpKind::Not, branch.cond.upcast_ty())
                .downcast_ty();
            leaves(
                vcx,
                branch.else_.downcast_ty(),
                vcx.mk_conj(&[guard, otherwise]),
                out,
            );
        }
        vir::ExprKindData::PredicateApp(app)
            if app.target == "indexed_p_Param"
                && app.args.len() == 4
                && stable(app.args[2])
                && stable(app.args[3]) =>
        {
            out.push((guard, app))
        }
        _ => {}
    }
}

struct ReplaceIndex<'vir> {
    name: &'vir str,
    value: vir::ExprInt<'vir>,
}
impl<'vir> Folder<'vir, (), !> for ReplaceIndex<'vir> {
    fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
        if let vir::ExprKindData::Local(local) = e.kind
            && local.name == self.name
        {
            Some(self.value.as_dyn())
        } else {
            e.super_fold_with(self)
        }
    }
}

impl TaskEncoder for IndexedSeparationEnc {
    task_encoder::encoder_cache!(IndexedSeparationEnc);
    const ENCODER_NAME: &'static str = "checked indexed scalar separation encoder";
    type TaskDescription<'vir> = IndexedSeparationTask<'vir>;
    type OutputFullDependency<'vir> = IndexedSeparation<'vir>;
    type OutputFullLocal<'vir> = Vec<vir::Method<'vir>>;
    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        *task
    }
    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let element = task.projection.base();
        let params = deps.require_dep::<GenericParamsEnc>(element.args.context())?;
        // No distinctness claim for opaque, aggregate, or zero-sized referents.
        if !matches!(task.scalar.ty.specifics, TySpecifics::Primitive(_)) {
            return Ok((
                Vec::new(),
                IndexedSeparation {
                    methods: Vec::new(),
                    params,
                },
            ));
        }
        let indirect = deps.require_dep::<IndirectPredicatesEnc>(task.projection)?;
        let element_cast = deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(
            (!element.ty.specifics.is_param()).then_some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: element,
            }),
        )?;
        let scalar_cast =
            deps.require_dep::<GArgsCastEnc<crate::encoders::Impure>>(Some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: task.scalar,
            }))?;
        let physical = deps.require_dep::<TyUseImpureEnc>(task.scalar)?;
        let scalar_type = deps.require_dep::<TyExprEnc>(task.scalar)?;
        vir::with_vcx(|vcx| {
            let seq_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
            let sequence = vcx.mk_local_decl("separation_elements", seq_ty);
            let start = vcx.mk_local_decl("separation_start", vir::TYPE_INT);
            let raw = vcx.mk_local_decl("separation_raw", vir::TYPE_REF);
            let k = vcx.mk_local_decl("separation_k", vir::TYPE_INT);
            let seq = vcx.mk_local_ex(sequence);
            let from = vcx.mk_local_ex(start);
            let r = vcx.mk_local_ex(raw);
            let index = vcx.mk_local_ex(k);
            let item = vcx
                .mk_seq_index_expr(seq, index)
                .downcast_ty::<vir::PSnap>();
            let snap = element_cast.cast_to_caller_ctx(item.upcast_ty());
            let plain = indirect
                .predicate_applications
                .iter()
                .map(|p| p.reify(vcx, ExprInput::current(snap)))
                .collect::<Vec<_>>();
            let indexed = vcx.alloc(IndexedResources::new(deps, vcx.mk_conj(&plain))?);
            let expressions = indirect
                .predicate_applications
                .iter()
                .map(|p| {
                    p.reify(
                        vcx,
                        ExprInput {
                            indexed: Some((indexed, index, true)),
                            ..ExprInput::current(snap)
                        },
                    )
                })
                .collect::<Vec<_>>();
            let mut selected = Vec::new();
            leaves(
                vcx,
                vcx.mk_conj(&expressions),
                vcx.mk_bool::<true>(),
                &mut selected,
            );
            let raw_pred = vcx.alloc(vir::PredicateAppData {
                target: "p_Param",
                args: vcx.alloc_slice(&[r.as_dyn(), scalar_type.as_dyn()]),
                perm: None,
            });
            let raw_permission = vcx.mk_predicate_app_expr(raw_pred);
            let observer = vir::FunctionIdn::<(vir::Ref, vir::TyVal), vir::PSnap>::new(
                vir::ViperIdent::new("p_Param_snap"),
                (vir::TYPE_REF, vir::TYPE_TYVAL),
                vir::TYPE_PSNAP,
            );
            let raw_state = observer.call()(r, scalar_type);
            let length = vcx.mk_collection_len_expr(seq);
            let zero = vcx.mk_uint::<0>();
            let next = (vir::expr! { (from) + ([vcx.mk_uint::<1>()]) }).downcast_ty::<vir::Int>();
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            format!("{task:?}").hash(&mut hash);
            let suffix = hash.finish();
            let mut methods = Vec::new();
            let mut definitions = Vec::new();
            for (leaf_index, (guard, app)) in selected.into_iter().enumerate() {
                let guard = vcx.mk_conj(&[
                    guard,
                    vcx.mk_eq_expr(app.args[3].downcast_ty::<vir::TyVal>(), scalar_type),
                ]);
                let at = |i| {
                    let mut replace = ReplaceIndex {
                        name: k.name,
                        value: i,
                    };
                    let condition = guard.fold_with(&mut replace).unwrap_or(guard);
                    let resource = app.fold_with(&mut replace).unwrap_or(app);
                    (condition, resource)
                };
                let quantify = |body| {
                    vcx.mk_forall_expr(
                        vcx.alloc_slice(&[k.as_dyn()]),
                        vcx.alloc_slice(&[vcx.mk_trigger(&[item.as_dyn()])]),
                        body,
                    )
                };
                let range = vir::expr! { (((from) <= (index)) && ((index) < (length))) && (guard) };
                let resource = vcx.mk_predicate_app_expr(app);
                let state = vcx.mk_unfolding_expr(
                    app,
                    observer.call()(app.args[2].downcast_ty(), scalar_type),
                );
                let resources = quantify(vir::expr! { (range) ==> (resource) });
                let frame =
                    quantify(vir::expr! { (range) ==> ((state) == ([vcx.mk_old_expr(state)])) });
                let distinct = quantify(
                    vir::expr! { (range) ==> ([vcx.mk_bin_op_expr(vir::BinOpKind::CmpNe, app.args[2].downcast_ty::<vir::Ref>(), r).downcast_ty::<vir::Bool>()]) },
                );
                let method = MethodIdn::<Arguments>::new(
                    vir::vir_format_identifier!(
                        vcx,
                        "indexed_scalar_separate_{suffix:x}_{leaf_index}"
                    ),
                    (
                        seq_ty,
                        vir::TYPE_INT,
                        vir::TYPE_REF,
                        params.ty_args(),
                        params.const_args(),
                    ),
                );
                let (condition, current) = at(from);
                let address = current.args[2].downcast_ty::<vir::Ref>();
                let mut body = vec![vcx.mk_unfold_stmt(current)];
                body.extend(scalar_cast.cast_to_caller_ctx(address));
                body.extend(scalar_cast.cast_to_caller_ctx(r));
                body.push(vcx.mk_unfold_stmt(physical.ref_to_pred_app(address, None)));
                body.push(vcx.mk_unfold_stmt(physical.ref_to_pred_app(r, None)));
                body.push(
                    vcx.mk_assert_stmt(
                        vcx.mk_bin_op_expr(vir::BinOpKind::CmpNe, address, r)
                            .downcast_ty(),
                    ),
                );
                body.push(vcx.mk_fold_stmt(physical.ref_to_pred_app(address, None)));
                body.push(vcx.mk_fold_stmt(physical.ref_to_pred_app(r, None)));
                body.extend(scalar_cast.cast_to_callee_ctx(address));
                body.extend(scalar_cast.cast_to_callee_ctx(r));
                body.push(vcx.mk_fold_stmt(current));
                let step = [
                    vcx.mk_if_stmt(condition, vcx.alloc_slice(&body), &[]),
                    method(seq, next, r, params.ty_exprs(), params.const_exprs()).alloc(),
                ];
                let statements = [vcx.mk_if_stmt(
                    vir::expr! { (from) < (length) },
                    vcx.alloc_slice(&step),
                    &[],
                )];
                let block = vcx.mk_cfg_block(
                    &vir::CfgBlockLabelData::Start,
                    &[],
                    vcx.alloc_slice(&statements),
                    &vir::TerminatorStmtGenData::Exit,
                );
                let pres = [
                    vir::expr! { ((zero) <= (from)) && ((from) <= (length)) },
                    raw_permission,
                    resources,
                ];
                let posts = [
                    raw_permission,
                    resources,
                    vcx.mk_eq_expr(raw_state, vcx.mk_old_expr(raw_state)),
                    frame,
                    distinct,
                ];
                definitions.push(vcx.mk_method(
                    method,
                    (
                        sequence,
                        start,
                        raw,
                        params.ty_decls(),
                        params.const_decls(),
                    ),
                    &[],
                    vcx.alloc_slice(&pres),
                    vcx.alloc_slice(&posts),
                    Some(vcx.alloc_slice(&[block])),
                ));
                methods.push(method);
            }
            Ok((definitions, IndexedSeparation { methods, params }))
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
