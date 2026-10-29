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
            let path = vcx.mk_local_decl("path", vcx.mk_ty_seq(vir::TYPE_INT));
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
            let params = [index.as_dyn(), path.as_dyn()]
                .into_iter()
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
    /// Retain the identities exposed by an indexed expansion. Only executable,
    /// permission-checked reads initialise these locals; guards are captured
    /// before any branch-specific reads.
    pub fn capture(
        expression: vir::ExprBool<'vir>,
        fresh: &mut impl FnMut(vir::TypeDyn<'vir>) -> vir::ExprDyn<'vir>,
    ) -> Result<(vir::ExprBool<'vir>, Vec<vir::Stmt<'vir>>), &'static str> {
        vir::with_vcx(|vcx| {
            Ok(match expression.kind {
                vir::ExprKindData::PredicateApp(app) => {
                    let mut statements = Vec::new();
                    let args = app
                        .args
                        .iter()
                        .map(|arg| {
                            let value = fresh(arg.ty());
                            statements.push(vcx.mk_pure_assign_stmt(value, *arg));
                            value
                        })
                        .collect::<Vec<_>>();
                    (
                        vcx.mk_predicate_app_expr(vcx.alloc(vir::PredicateAppData {
                            args: vcx.alloc_slice(&args),
                            target: app.target,
                            perm: app.perm,
                        })),
                        statements,
                    )
                }
                vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => {
                    let (left, mut statements) = Self::capture(op.lhs.downcast_ty(), fresh)?;
                    let (right, rest) = Self::capture(op.rhs.downcast_ty(), fresh)?;
                    statements.extend(rest);
                    (vcx.mk_conj(&[left, right]), statements)
                }
                vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::Implies => {
                    let condition = fresh(vir::TYPE_BOOL.upcast_ty());
                    let (body, statements) = Self::capture(op.rhs.downcast_ty(), fresh)?;
                    (
                        vir::expr! { ([condition.downcast_ty::<vir::Bool>()]) ==> (body) },
                        vec![
                            vcx.mk_pure_assign_stmt(condition, op.lhs),
                            vcx.mk_if_stmt(
                                condition.downcast_ty(),
                                vcx.alloc_slice(&statements),
                                &[],
                            ),
                        ],
                    )
                }
                vir::ExprKindData::Ternary(expr) => {
                    let condition = fresh(vir::TYPE_BOOL.upcast_ty());
                    let (yes, yes_statements) = Self::capture(expr.then.downcast_ty(), fresh)?;
                    let (no, no_statements) = Self::capture(expr.else_.downcast_ty(), fresh)?;
                    (
                        vcx.mk_ternary_expr(condition.downcast_ty(), yes, no),
                        vec![
                            vcx.mk_pure_assign_stmt(condition, expr.cond.upcast_ty()),
                            vcx.mk_if_stmt(
                                condition.downcast_ty(),
                                vcx.alloc_slice(&yes_statements),
                                vcx.alloc_slice(&no_statements),
                            ),
                        ],
                    )
                }
                vir::ExprKindData::Forall(_) => return Err("nested quantified element resources"),
                _ => (expression, Vec::new()),
            })
        })
    }

    /// Import exposed resources from the enclosing heap before a package's
    /// permission-dependent folds inspect their representation. Wrapped
    /// resources belong to the wand's antecedent and are opened by its script.
    pub fn exposed_footprint(
        &self,
        expression: vir::ExprBool<'vir>,
        label: &'vir str,
    ) -> vir::ExprBool<'vir> {
        struct Frame<'a, 'vir>(&'a IndexedResources<'vir>, &'vir str);
        impl<'vir> Folder<'vir, (), !> for Frame<'_, 'vir> {
            fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                if let vir::ExprKindData::PredicateApp(app) = expr.kind {
                    let app = self.0.unwrapped(app).unwrap_or(app);
                    Some(vir::with_vcx(|vcx| {
                        let held =
                            vcx.mk_local_labelled_old_expr(vcx.mk_predicate_perm_expr(app), self.1);
                        let none = vcx.mk_no_perm();
                        let resource = vcx.mk_predicate_app_expr(app);
                        vir::expr! { ((none) < (held)) ==> (resource) }.upcast_ty()
                    }))
                } else {
                    expr.super_fold_with(self)
                }
            }
        }
        expression
            .fold_with(&mut Frame(self, label))
            .unwrap_or(expression)
    }

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
        path: &'vir [usize],
        app: vir::PredicateApp<'vir>,
    ) -> vir::PredicateApp<'vir> {
        vir::with_vcx(|vcx| {
            vcx.alloc(vir::PredicateAppData {
                target: self.names[app.target],
                args: vcx.alloc_slice(
                    &[
                        index.as_dyn(),
                        vcx.mk_seq_literal_expr(
                            vcx.alloc_slice(
                                &path
                                    .iter()
                                    .map(|step| {
                                        vcx.mk_const_expr(vir::ConstData::Int(*step as u128))
                                            .downcast_ty::<vir::Int>()
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            vir::TYPE_INT,
                        )
                        .as_dyn(),
                    ]
                    .into_iter()
                    .chain(app.args.iter().copied())
                    .collect::<Vec<_>>(),
                ),
                perm: app.perm,
            })
        })
    }

    fn unwrapped(&self, app: vir::PredicateApp<'vir>) -> Option<vir::PredicateApp<'vir>> {
        let target = self
            .names
            .iter()
            .find_map(|(raw, indexed)| (*indexed == app.target).then_some(*raw))?;
        Some(vir::with_vcx(|vcx| {
            vcx.alloc(vir::PredicateAppData {
                target,
                args: &app.args[2..],
                perm: app.perm,
            })
        }))
    }

    /// Read through a resource in either its exposed or indexed form. The
    /// unfolding expression preserves the wrapper while establishing the
    /// snapshot function's permission precondition.
    pub fn read_snapshot(
        &self,
        app: vir::PredicateApp<'vir>,
        snapshot: vir::ExprSnap<'vir>,
        index: vir::ExprInt<'vir>,
        inspect_permissions: bool,
        path: &'vir [usize],
    ) -> vir::ExprSnap<'vir> {
        if !self.names.contains_key(app.target) {
            return snapshot;
        }
        vir::with_vcx(|vcx| {
            let wrapped = vcx.mk_unfolding_expr(self.application(index, path, app), snapshot);
            // Contract resources have a fixed representation. Viper rejects
            // permission inspection in ordinary pre/postconditions.
            if !inspect_permissions {
                return wrapped;
            }
            let held = vcx.mk_predicate_perm_expr(app);
            let none = vcx.mk_no_perm();
            vcx.mk_ternary_expr(vir::expr! { (none) < (held) }, snapshot, wrapped)
        })
    }

    pub fn wrap(
        &self,
        expression: vir::ExprBool<'vir>,
        index: vir::ExprInt<'vir>,
        path: &'vir [usize],
    ) -> vir::ExprBool<'vir> {
        struct Wrap<'a, 'vir>(
            &'a IndexedResources<'vir>,
            vir::ExprInt<'vir>,
            &'vir [usize],
        );
        impl<'vir> Folder<'vir, (), !> for Wrap<'_, 'vir> {
            fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                if let vir::ExprKindData::PredicateApp(app) = expr.kind {
                    Some(vir::with_vcx(|vcx| {
                        vcx.mk_predicate_app_expr(self.0.application(self.1, self.2, app))
                            .as_dyn()
                    }))
                } else {
                    expr.super_fold_with(self)
                }
            }
        }
        expression
            .fold_with(&mut Wrap(self, index, path))
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
        self.operations_with_strategy(expression, index, fold, false)
    }

    pub fn open_future(
        &self,
        expression: vir::ExprBool<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> Result<Vec<vir::Stmt<'vir>>, &'static str> {
        self.operations_with_strategy(expression, index, false, true)
    }

    fn operations_with_strategy(
        &self,
        expression: vir::ExprBool<'vir>,
        index: vir::ExprInt<'vir>,
        fold: bool,
        future: bool,
    ) -> Result<Vec<vir::Stmt<'vir>>, &'static str> {
        vir::with_vcx(|vcx| {
            Ok(match expression.kind {
                vir::ExprKindData::PredicateApp(app) => {
                    let (app, wrapped) = match self.unwrapped(app) {
                        Some(raw) => (raw, *app),
                        None => (*app, self.application(index, &[], app)),
                    };
                    // Several historical expansions may expose the same referent.
                    // Select its existing representation, and check the alternative
                    // explicitly; absence of both representations must fail.
                    let held = vcx.mk_predicate_perm_expr(if future { app } else { wrapped });
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
                    let (yes, no) = if future || fold {
                        (unchanged, change)
                    } else {
                        (change, unchanged)
                    };
                    vec![vcx.mk_if_stmt(condition, vcx.alloc_slice(&[yes]), vcx.alloc_slice(&[no]))]
                }
                vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => {
                    let mut stmts =
                        self.operations_with_strategy(op.lhs.downcast_ty(), index, fold, future)?;
                    stmts.extend(self.operations_with_strategy(
                        op.rhs.downcast_ty(),
                        index,
                        fold,
                        future,
                    )?);
                    stmts
                }
                vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::Implies => {
                    let body =
                        self.operations_with_strategy(op.rhs.downcast_ty(), index, fold, future)?;
                    vec![vcx.mk_if_stmt(op.lhs.downcast_ty(), vcx.alloc_slice(&body), &[])]
                }
                vir::ExprKindData::Ternary(expr) => {
                    let yes = self.operations_with_strategy(
                        expr.then.downcast_ty(),
                        index,
                        fold,
                        future,
                    )?;
                    let no = self.operations_with_strategy(
                        expr.else_.downcast_ty(),
                        index,
                        fold,
                        future,
                    )?;
                    vec![vcx.mk_if_stmt(expr.cond, vcx.alloc_slice(&yes), vcx.alloc_slice(&no))]
                }
                vir::ExprKindData::Forall(_) => return Err("nested quantified element resources"),
                // Pure assertions do not require a fold/unfold operation.
                _ => Vec::new(),
            })
        })
    }
}
