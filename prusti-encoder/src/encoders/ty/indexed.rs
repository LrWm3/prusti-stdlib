//! Indexed wrappers for indirect resources. The index makes quantified
//! predicate instances injective without assuming distinct pointer addresses.
use rustc_hash::FxHashMap;
use task_encoder::{EncodeFullError, EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::{CastType, Foldable, Folder};

/// Silicon's API expects quantified resources in atomic form. Its parser
/// performs this distribution, but Prusti constructs the Viper AST directly.
pub(crate) fn quantify_resources<'vir>(
    vcx: &'vir vir::VirCtxt<'_>,
    variables: &'vir [vir::LocalDeclDyn<'vir>],
    triggers: &'vir [vir::Trigger<'vir>],
    condition: vir::ExprBool<'vir>,
    expression: vir::ExprBool<'vir>,
) -> vir::ExprBool<'vir> {
    match expression.kind {
        vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => vcx.mk_conj(&[
            quantify_resources(vcx, variables, triggers, condition, op.lhs.downcast_ty()),
            quantify_resources(vcx, variables, triggers, condition, op.rhs.downcast_ty()),
        ]),
        vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::Implies => quantify_resources(
            vcx,
            variables,
            triggers,
            vcx.mk_conj(&[condition, op.lhs.downcast_ty()]),
            op.rhs.downcast_ty(),
        ),
        vir::ExprKindData::Ternary(expr) => vcx.mk_conj(&[
            quantify_resources(
                vcx,
                variables,
                triggers,
                vcx.mk_conj(&[condition, expr.cond]),
                expr.then.downcast_ty(),
            ),
            quantify_resources(
                vcx,
                variables,
                triggers,
                vcx.mk_conj(&[
                    condition,
                    vcx.mk_unary_op_expr(vir::UnOpKind::Not, expr.cond.upcast_ty())
                        .downcast_ty(),
                ]),
                expr.else_.downcast_ty(),
            ),
        ]),
        vir::ExprKindData::Forall(inner) => {
            let variables = vcx.alloc_slice(
                &variables
                    .iter()
                    .chain(inner.qvars)
                    .copied()
                    .collect::<Vec<_>>(),
            );
            let terms = triggers
                .iter()
                .chain(inner.triggers)
                .flat_map(|trigger| trigger.exprs.iter().copied())
                .collect::<Vec<_>>();
            let triggers = vcx.alloc_slice(&[vcx.mk_trigger(&terms)]);
            quantify_resources(vcx, variables, triggers, condition, inner.body)
        }
        _ => vcx.mk_forall_expr(
            variables,
            triggers,
            vir::expr! { (condition) ==> (expression) },
        ),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexedPredicateTask<'vir> {
    target: &'vir str,
    args: Vec<vir::TypeDyn<'vir>>,
}

pub(crate) struct IndexedPredicateEnc;

impl TaskEncoder for IndexedPredicateEnc {
    task_encoder::encoder_cache!(IndexedPredicateEnc);
    const ENCODER_NAME: &'static str = "indexed resource predicate encoder";
    type TaskDescription<'vir> = IndexedPredicateTask<'vir>;
    type OutputFullLocal<'vir> = vir::Predicate<'vir>;
    type OutputFullDependency<'vir> = &'vir str;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        task.clone()
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(task.clone(), ())?;
        vir::with_vcx(|vcx| {
            let name = vir::vir_format!(vcx, "indexed_{}", task.target);
            let index = vcx.mk_local_decl("index", vir::TYPE_INT);
            let args = task
                .args
                .iter()
                .enumerate()
                .map(|(i, ty)| vcx.mk_local_decl(vir::vir_format!(vcx, "arg{i}"), *ty))
                .collect::<Vec<_>>();
            let body = vcx.mk_predicate_app_expr(
                vcx.alloc(vir::PredicateAppData {
                    target: task.target,
                    args: vcx.alloc_slice(
                        &args
                            .iter()
                            .map(|arg| vcx.mk_local_ex(*arg))
                            .collect::<Vec<_>>(),
                    ),
                    perm: None,
                }),
            );
            let params = std::iter::once(index.as_dyn())
                .chain(args)
                .collect::<Vec<_>>();
            Ok((
                vcx.mk_predicate_unchecked(name, vcx.alloc_slice(&params), Some(body)),
                name,
            ))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for predicate in Self::all_outputs_local_no_errors(program) {
            program.add_predicate(predicate);
        }
    }
}

#[derive(Clone)]
pub(crate) struct IndexedResources<'vir> {
    names: FxHashMap<&'vir str, &'vir str>,
}

impl<'vir> IndexedResources<'vir> {
    pub fn new<E: TaskEncoder>(
        deps: &mut TaskEncoderDependencies<'vir, E>,
        expression: vir::ExprBool<'vir>,
    ) -> Result<Self, EncodeFullError<'vir, E>> {
        struct Collect<'vir>(Vec<vir::PredicateApp<'vir>>);
        impl<'vir> Folder<'vir, (), !> for Collect<'vir> {
            fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                if let vir::ExprKindData::PredicateApp(app) = expr.kind {
                    self.0.push(app);
                    None
                } else {
                    expr.super_fold_with(self)
                }
            }
        }
        let mut collect = Collect(Vec::new());
        expression.fold_with(&mut collect);
        let mut names = FxHashMap::default();
        for app in collect.0 {
            let name = deps.require_dep::<IndexedPredicateEnc>(IndexedPredicateTask {
                target: app.target,
                args: app.args.iter().map(|arg| arg.ty()).collect(),
            })?;
            names.insert(app.target, name);
        }
        Ok(Self { names })
    }

    fn application(
        &self,
        index: vir::ExprInt<'vir>,
        app: vir::PredicateApp<'vir>,
    ) -> vir::PredicateApp<'vir> {
        vir::with_vcx(|vcx| {
            vcx.alloc(vir::PredicateAppData {
                target: self.names[app.target],
                args: vcx.alloc_slice(
                    &std::iter::once(index.as_dyn())
                        .chain(app.args.iter().copied())
                        .collect::<Vec<_>>(),
                ),
                perm: app.perm,
            })
        })
    }

    pub fn wrap(
        &self,
        expression: vir::ExprBool<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> vir::ExprBool<'vir> {
        struct Wrap<'a, 'vir>(&'a IndexedResources<'vir>, vir::ExprInt<'vir>);
        impl<'vir> Folder<'vir, (), !> for Wrap<'_, 'vir> {
            fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                if let vir::ExprKindData::PredicateApp(app) = expr.kind {
                    Some(vir::with_vcx(|vcx| {
                        vcx.mk_predicate_app_expr(self.0.application(self.1, app))
                            .as_dyn()
                    }))
                } else {
                    expr.super_fold_with(self)
                }
            }
        }
        expression
            .fold_with(&mut Wrap(self, index))
            .unwrap_or(expression)
    }

    /// Open/close the wrappers for a selected element. Variant guards stay
    /// outside the operations, so inactive fields require no resources.
    pub fn operations(
        &self,
        expression: vir::ExprBool<'vir>,
        index: vir::ExprInt<'vir>,
        fold: bool,
    ) -> Result<Vec<vir::Stmt<'vir>>, &'static str> {
        vir::with_vcx(|vcx| {
            Ok(match expression.kind {
                vir::ExprKindData::PredicateApp(app) => {
                    let wrapped = self.application(index, app);
                    // Several historical expansions may expose the same referent.
                    // Select its existing representation, and check the alternative
                    // explicitly; absence of both representations must fail.
                    let held = vcx.mk_predicate_perm_expr(if fold { app } else { wrapped });
                    let none = vcx.mk_no_perm();
                    let condition = vir::expr! { (none) < (held) };
                    let change = if fold {
                        vcx.mk_fold_stmt(wrapped)
                    } else {
                        vcx.mk_unfold_stmt(wrapped)
                    };
                    let unchanged = vcx.mk_assert_stmt(vcx.mk_predicate_app_expr(if fold {
                        wrapped
                    } else {
                        app
                    }));
                    vec![vcx.mk_if_stmt(
                        condition,
                        vcx.alloc_slice(&[change]),
                        vcx.alloc_slice(&[unchanged]),
                    )]
                }
                vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => {
                    let mut stmts = self.operations(op.lhs.downcast_ty(), index, fold)?;
                    stmts.extend(self.operations(op.rhs.downcast_ty(), index, fold)?);
                    stmts
                }
                vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::Implies => {
                    let body = self.operations(op.rhs.downcast_ty(), index, fold)?;
                    vec![vcx.mk_if_stmt(op.lhs.downcast_ty(), vcx.alloc_slice(&body), &[])]
                }
                vir::ExprKindData::Ternary(expr) => {
                    let yes = self.operations(expr.then.downcast_ty(), index, fold)?;
                    let no = self.operations(expr.else_.downcast_ty(), index, fold)?;
                    vec![vcx.mk_if_stmt(expr.cond, vcx.alloc_slice(&yes), vcx.alloc_slice(&no))]
                }
                vir::ExprKindData::Forall(_) => return Err("nested quantified element resources"),
                // Pure assertions do not require a fold/unfold operation.
                _ => Vec::new(),
            })
        })
    }
}
