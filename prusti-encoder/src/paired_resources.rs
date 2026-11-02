//! Lower flat resource continuations to two separately checked half packages.
//!
//! This is a representation of the continuation, not a rule for splitting an
//! arbitrary existing wand. Contracts, invariants, packages and applications
//! are lowered together after substitution. Each ordinary wand instance owns
//! half permissions in its operands; the wand itself is never fractional.
use vir::{CastType, Foldable, Folder};

fn full(app: vir::PredicateApp<'_>) -> bool {
    app.perm
        .is_none_or(|p| matches!(p.kind, vir::ExprKindData::Const(vir::ConstData::FullPerm)))
}

fn leaf<'vir>(expr: vir::ExprBool<'vir>) -> Option<vir::PredicateApp<'vir>> {
    fn collect<'vir>(expr: vir::ExprBool<'vir>, leaves: &mut Vec<vir::PredicateApp<'vir>>) -> bool {
        use vir::ExprKindData as K;
        match expr.kind {
            K::PredicateApp(app)
                if matches!(app.target, "p_Param" | "indirect_resources") && full(app) =>
            {
                leaves.push(app);
                true
            }
            K::BinOp(op) if op.kind == vir::BinOpKind::And => {
                collect(op.lhs.downcast_ty(), leaves) && collect(op.rhs.downcast_ty(), leaves)
            }
            _ => {
                struct Pure(bool);
                impl<'vir> Folder<'vir, (), !> for Pure {
                    fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                        if matches!(
                            e.kind,
                            K::PredicateApp(_)
                                | K::AccField(_)
                                | K::PredicatePerm(_)
                                | K::Wand(_)
                                | K::InhaleExhale(_)
                        ) {
                            self.0 = false;
                        }
                        e.super_fold_with(self)
                    }
                }
                let mut pure = Pure(true);
                let _ = expr.fold_with(&mut pure);
                pure.0
            }
        }
    }
    let mut leaves = Vec::new();
    (collect(expr, &mut leaves) && leaves.len() == 1).then(|| leaves[0])
}

fn paired(wand: vir::Wand<'_>) -> bool {
    match (leaf(wand.lhs), leaf(wand.rhs)) {
        (Some(lhs), Some(rhs)) => lhs.target == rhs.target,
        _ => false,
    }
}

struct Lower<'vir> {
    vcx: &'vir vir::VirCtxt<'vir>,
    half: bool,
}

impl<'vir> Lower<'vir> {
    fn predicate(&self, app: vir::PredicateApp<'vir>) -> vir::PredicateApp<'vir> {
        if !self.half || !full(app) {
            return app;
        }
        let write = self
            .vcx
            .mk_const_expr(vir::ConstData::FullPerm)
            .downcast_ty::<vir::Perm>();
        let two = self
            .vcx
            .mk_bin_op_expr(vir::BinOpKind::PermAdd, write, write);
        let half = self
            .vcx
            .mk_bin_op_expr(
                vir::BinOpKind::PermPermDiv,
                write,
                two.downcast_ty::<vir::Perm>(),
            )
            .downcast_ty();
        self.vcx.alloc(vir::PredicateAppData {
            perm: Some(half),
            ..*app
        })
    }

    fn wand(&self, wand: vir::Wand<'vir>, half: bool) -> vir::Wand<'vir> {
        let mut lower = Lower {
            vcx: self.vcx,
            half,
        };
        // Fold the operands, avoiding re-selection of this same wand.
        wand.super_fold_with(&mut lower).unwrap_or(wand)
    }

    fn statements(&mut self, stmts: &[vir::Stmt<'vir>]) -> &'vir [vir::Stmt<'vir>] {
        use vir::StmtKindData as S;
        let mut result = Vec::new();
        for &stmt in stmts {
            let mut push = |kind| {
                result.push(self.vcx.alloc(vir::StmtData {
                    kind: self.vcx.alloc(kind),
                    span: stmt.span,
                }))
            };
            match stmt.kind {
                S::Package(wand, body) if paired(wand) => {
                    let wand = self.wand(wand, true);
                    let body = Lower {
                        vcx: self.vcx,
                        half: true,
                    }
                    .statements(body);
                    push(S::Package(wand, body));
                    if !self.half {
                        push(S::Package(wand, body));
                    }
                }
                S::Package(wand, body) => {
                    let wand = self.wand(wand, self.half);
                    let body = self.statements(body);
                    push(S::Package(wand, body));
                }
                S::Apply(wand) if paired(wand) => {
                    let wand = self.wand(wand, true);
                    push(S::Apply(wand));
                    if !self.half {
                        push(S::Apply(wand));
                    }
                }
                S::If(cond, yes, no) => {
                    let cond = cond.fold_with(self).unwrap_or(cond);
                    let yes = self.statements(yes);
                    let no = self.statements(no);
                    push(S::If(cond, yes, no));
                }
                S::Fold(app) | S::Unfold(app) => {
                    let app = app.fold_with(self).unwrap_or(app);
                    let app = self.predicate(app);
                    push(if matches!(stmt.kind, S::Fold(_)) {
                        S::Fold(app)
                    } else {
                        S::Unfold(app)
                    });
                }
                // Calls keep their original checked contracts. A conversion
                // requiring full ownership cannot silently run on a half.
                _ => result.push(stmt.fold_with(self).unwrap_or(stmt)),
            }
        }
        self.vcx.alloc_slice(&result)
    }

    fn method(&mut self, method: vir::Method<'vir>) -> vir::Method<'vir> {
        let pres = method.pres.fold_with(self).unwrap_or(method.pres);
        let posts = method.posts.fold_with(self).unwrap_or(method.posts);
        let body = method.body.map(|body| {
            let blocks = body
                .blocks
                .iter()
                .map(|block| {
                    let label = block.label.fold_with(self).unwrap_or(block.label);
                    let stmts = self.statements(block.stmts);
                    let terminator = match block.terminator {
                        vir::TerminatorStmtData::GotoIf(branch) => {
                            let branch = branch.fold_with(self).unwrap_or(branch);
                            let targets = branch
                                .targets
                                .iter()
                                .map(|target| {
                                    self.vcx.alloc(vir::GotoIfTargetGenData {
                                        statements: self.statements(target.statements),
                                        ..**target
                                    })
                                })
                                .collect::<Vec<_>>();
                            let branch = self.vcx.alloc(vir::GotoIfData {
                                targets: self.vcx.alloc_slice(&targets),
                                otherwise_statements: self.statements(branch.otherwise_statements),
                                ..*branch
                            });
                            self.vcx.alloc(vir::TerminatorStmtData::GotoIf(branch))
                        }
                        _ => block.terminator,
                    };
                    self.vcx.alloc(vir::CfgBlockData {
                        label,
                        stmts,
                        terminator,
                    })
                })
                .collect::<Vec<_>>();
            self.vcx.alloc(vir::MethodBodyData {
                blocks: self.vcx.alloc_slice(&blocks),
            })
        });
        self.vcx.alloc(vir::MethodData {
            pres,
            posts,
            body,
            ..*method
        })
    }
}

impl<'vir> Folder<'vir, (), !> for Lower<'vir> {
    fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
        use vir::ExprKindData as K;
        if let K::Wand(wand) = expr.kind {
            if paired(wand) {
                let wand = self.wand(wand, true);
                let once = self.vcx.mk_wand_expr(wand);
                // Assertions within a half package need only the half instance
                // they consume. Full contracts and invariants retain both.
                let replacement = if self.half {
                    once
                } else {
                    self.vcx.mk_conj(&[once, once])
                };
                return Some(expr.with_kind(replacement.kind));
            }
        }
        let folded = expr.super_fold_with(self).unwrap_or(expr);
        let kind = match folded.kind {
            K::PredicateApp(app) if self.half && full(app) => K::PredicateApp(self.predicate(app)),
            K::Unfolding(unfold) if self.half && full(unfold.target) => {
                K::Unfolding(self.vcx.alloc(vir::UnfoldingData {
                    target: self.predicate(unfold.target),
                    ..**unfold
                }))
            }
            _ => return (!std::ptr::eq(folded, expr)).then_some(folded),
        };
        Some(expr.with_kind(self.vcx.alloc(kind)))
    }
}

pub(crate) fn lower<'vir>(program: vir::Program<'vir>) -> vir::Program<'vir> {
    vir::with_vcx(|vcx| {
        let mut lower = Lower { vcx, half: false };
        let methods = program
            .methods
            .iter()
            .map(|method| lower.method(method))
            .collect::<Vec<_>>();
        vcx.alloc(vir::ProgramData {
            methods: vcx.alloc_slice(&methods),
            ..*program
        })
    })
}

/// Compose a linear opaque history before restoring a concrete flat footprint.
/// The old and new physical resources can overlap even if an intermediate
/// owner's footprint does not. Keeping a current half during the first apply,
/// and an entry half during the second, frames the overlapping observations.
///
/// Use the boundary's existing checked folds/unfolds as concrete views. More
/// complex conditional histories, method conversions and non-flat footprints
/// retain their existing proof. This replaces a proof script only: the complete
/// original wand consequent, including every value frame and pledge, is checked.
pub(crate) fn compose_flat_restoration<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    wand: vir::Wand<'vir>,
    original: &[vir::Stmt<'vir>],
    fresh: &mut usize,
) -> Option<Vec<vir::Stmt<'vir>>> {
    use vir::{ExprKindData as K, StmtKindData as S};
    fn physical<'vir>(expr: vir::ExprBool<'vir>, out: &mut Vec<vir::PredicateApp<'vir>>) -> bool {
        match expr.kind {
            K::PredicateApp(app) if app.target == "p_Param" && full(app) => {
                out.push(app);
                true
            }
            K::BinOp(op) if op.kind == vir::BinOpKind::And => {
                physical(op.lhs.downcast_ty(), out) && physical(op.rhs.downcast_ty(), out)
            }
            _ => {
                struct Pure(bool);
                impl<'vir> Folder<'vir, (), !> for Pure {
                    fn fold_expr(&mut self, e: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                        if matches!(
                            e.kind,
                            K::PredicateApp(_)
                                | K::AccField(_)
                                | K::PredicatePerm(_)
                                | K::Wand(_)
                                | K::InhaleExhale(_)
                        ) {
                            self.0 = false;
                        }
                        e.super_fold_with(self)
                    }
                }
                let mut pure = Pure(true);
                let _ = expr.fold_with(&mut pure);
                pure.0
            }
        }
    }
    fn empty(stmts: &[vir::Stmt<'_>]) -> bool {
        stmts.iter().all(|stmt| match stmt.kind {
            S::Comment(_) => true,
            S::If(_, yes, no) => empty(yes) && empty(no),
            _ => false,
        })
    }
    let mut current = Vec::new();
    let mut entry = Vec::new();
    if !physical(wand.lhs, &mut current)
        || !physical(wand.rhs, &mut entry)
        || current.len() < 2
        || entry.is_empty()
    {
        return None;
    }
    let mut script = Vec::new();
    for &stmt in original {
        match stmt.kind {
            S::If(_, yes, no) if empty(yes) && empty(no) => (),
            S::Fold(_) | S::Unfold(_) | S::Assert(_) | S::Comment(_) => script.push(stmt),
            S::Apply(step) if paired(step) && leaf(step.lhs)?.target == "indirect_resources" => {
                script.push(stmt)
            }
            _ => return None,
        }
    }
    let steps = script
        .iter()
        .enumerate()
        .filter_map(|(i, stmt)| {
            if let S::Apply(wand) = stmt.kind {
                Some((i, *wand))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if steps.len() < 2 {
        return None;
    }
    let &(first, first_wand) = steps.first()?;
    let &(last, last_wand) = steps.last()?;
    let prefix = &script[..first];
    let suffix = &script[last + 1..];
    if !prefix
        .iter()
        .any(|s| matches!(s.kind, S::Fold(p) if p.target == "indirect_resources"))
        || !suffix
            .iter()
            .any(|s| matches!(s.kind, S::Unfold(p) if p.target == "indirect_resources"))
    {
        return None;
    }
    let reverse_views = |stmts: &[vir::Stmt<'vir>]| {
        stmts
            .iter()
            .rev()
            .filter_map(|stmt| match stmt.kind {
                S::Fold(app) => Some(vcx.mk_unfold_stmt(app)),
                S::Unfold(app) => Some(vcx.mk_fold_stmt(app)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let observe = |app: vir::PredicateApp<'vir>| {
        vir::FunctionIdn::<(vir::Ref, vir::TyVal), vir::PSnap>::new(
            vir::ViperIdent::new("p_Param_snap"),
            (vir::TYPE_REF, vir::TYPE_TYVAL),
            vir::TYPE_PSNAP,
        )
        .call()(app.args[0].downcast_ty(), app.args[1].downcast_ty())
    };
    let mut result = Vec::new();
    let saved = current
        .iter()
        .map(|&app| {
            let name = vir::vir_format!(vcx, "_paired_current_{}", *fresh);
            *fresh += 1;
            let decl = vcx.mk_local_decl(name, vir::TYPE_PSNAP);
            result.push(vcx.mk_local_decl_stmt(decl, Some(observe(app))));
            vcx.mk_local_ex(decl)
        })
        .collect::<Vec<_>>();
    result.extend_from_slice(prefix);
    let composed = vcx.mk_wand(
        vcx.mk_predicate_app_expr(leaf(first_wand.lhs)?),
        vcx.mk_predicate_app_expr(leaf(last_wand.rhs)?),
    );
    let history = steps
        .iter()
        .map(|(_, step)| vcx.mk_apply_stmt(step))
        .collect::<Vec<_>>();
    // The common lowering pass produces two separately proved half packages.
    result.push(vcx.mk_package_stmt(composed, vcx.alloc_slice(&history)));
    // Explicit half applications leave space for concrete overlap coherence.
    let mut half = Lower { vcx, half: true };
    let partial = half.wand(composed, true);
    result.push(vcx.mk_apply_stmt(partial));
    let open_current = reverse_views(prefix);
    result.extend_from_slice(half.statements(&open_current));
    result.extend_from_slice(half.statements(suffix));
    for (&before, &value) in current.iter().zip(&saved) {
        for &after in &entry {
            let key = vcx.mk_conj(
                &before
                    .args
                    .iter()
                    .zip(after.args)
                    .map(|(&a, &b)| vcx.mk_eq_expr(a, b))
                    .collect::<Vec<_>>(),
            );
            let equal = vcx.mk_eq_expr(observe(after), value);
            result.push(
                vcx.mk_assert_stmt(
                    vcx.mk_bin_op_expr(vir::BinOpKind::Implies, key, equal)
                        .downcast_ty(),
                ),
            );
        }
    }
    result.extend_from_slice(half.statements(&reverse_views(suffix)));
    result.extend_from_slice(half.statements(&reverse_views(&open_current)));
    result.push(vcx.mk_apply_stmt(partial));
    result.extend_from_slice(suffix);
    Some(result)
}
