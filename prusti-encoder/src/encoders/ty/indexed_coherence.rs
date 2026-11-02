//! Checked coherence of differently indexed wrappers of the same resource.
use vir::{CastType, Foldable, Folder, MethodIdn};

type PairArgs = (vir::Int, vir::Int, vir::PSnap, vir::TyVal, vir::PSnap);
type RangeArgs = (vir::Seq, vir::Int, vir::Int, vir::TyVal, vir::PSnap);
type AllArgs = (vir::Seq, vir::Int, vir::TyVal, vir::PSnap);

fn all_method<'vir>(vcx: &'vir vir::VirCtxt<'vir>) -> MethodIdn<'vir, AllArgs> {
    MethodIdn::new(
        vir::ViperIdent::new("indexed_indirect_coherence_all"),
        (
            vcx.mk_ty_seq(vir::TYPE_PSNAP),
            vir::TYPE_INT,
            vir::TYPE_TYVAL,
            vir::TYPE_PSNAP,
        ),
    )
}

pub(crate) fn methods<'vir>(vcx: &'vir vir::VirCtxt<'vir>) -> Vec<vir::Method<'vir>> {
    let seq_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
    let values = vcx.mk_local_decl("coherence_values", seq_ty);
    let start = vcx.mk_local_decl("coherence_start", vir::TYPE_INT);
    let selected = vcx.mk_local_decl("coherence_selected", vir::TYPE_INT);
    let value = vcx.mk_local_decl("coherence_value", vir::TYPE_PSNAP);
    let typ = vcx.mk_local_decl("coherence_type", vir::TYPE_TYVAL);
    let metadata = vcx.mk_local_decl("coherence_metadata", vir::TYPE_PSNAP);
    let k = vcx.mk_local_decl("coherence_k", vir::TYPE_INT);
    let j = vcx.mk_local_decl("coherence_j", vir::TYPE_INT);
    let seq = vcx.mk_local_ex(values);
    let from = vcx.mk_local_ex(start);
    let sel = vcx.mk_local_ex(selected);
    let t = vcx.mk_local_ex(typ);
    let meta = vcx.mk_local_ex(metadata);
    let x = vcx.mk_local_ex(k);
    let y = vcx.mk_local_ex(j);
    let zero = vcx.mk_uint::<0>();
    let length = vcx.mk_collection_len_expr(seq);
    let element = |i| vcx.mk_seq_index_expr(seq, i).downcast_ty::<vir::PSnap>();
    let path = vcx.mk_seq_literal_expr(&[], vir::TYPE_INT);
    let wrapper = |i: vir::ExprInt<'vir>, v: vir::ExprPSnap<'vir>| {
        vcx.alloc(vir::PredicateAppData {
            target: "indexed_indirect_resources",
            args: vcx.alloc_slice(&[
                i.as_dyn(),
                path.as_dyn(),
                v.as_dyn(),
                t.as_dyn(),
                meta.as_dyn(),
            ]),
            perm: None,
        })
    };
    let permission = |i, v| vcx.mk_predicate_app_expr(wrapper(i, v));
    let observer = vir::FunctionIdn::<(vir::PSnap, vir::TyVal, vir::PSnap), vir::CSnap>::new(
        vir::ViperIdent::new("indirect_resource_state"),
        (vir::TYPE_PSNAP, vir::TYPE_TYVAL, vir::TYPE_PSNAP),
        super::resources::state_type(),
    );
    let state = |i, v| vcx.mk_unfolding_expr(wrapper(i, v), observer.call()(v, t, meta));
    let range = |i: vir::ExprInt<'vir>, lower: vir::ExprInt<'vir>| vir::expr! { ((lower) <= (i)) && ((i) < (length)) };
    let quantify = |body| {
        vcx.mk_forall_expr(
            vcx.alloc_slice(&[k.as_dyn()]),
            vcx.alloc_slice(&[vcx.mk_trigger(&[element(x).as_dyn()])]),
            body,
        )
    };
    let resources = |lower, exclude_selected: bool| {
        let guard = if exclude_selected {
            vir::expr! { ([range(x, lower)]) && ([vcx.mk_bin_op_expr(vir::BinOpKind::CmpNe, x, sel).downcast_ty::<vir::Bool>()]) }
        } else {
            range(x, lower)
        };
        quantify(vir::expr! { (guard) ==> ([permission(x, element(x))]) })
    };
    let frame = |lower| {
        quantify(vir::expr! { ([range(x, lower)]) ==>
        (([state(x, element(x))]) == ([vcx.mk_old_expr(state(x, element(x)))])) })
    };
    let block = |statements: Vec<vir::Stmt<'vir>>| {
        vcx.alloc_slice(&[vcx.mk_cfg_block(
            &vir::CfgBlockLabelData::Start,
            &[],
            vcx.alloc_slice(&statements),
            &vir::TerminatorStmtGenData::Exit,
        )])
    };
    let pair = MethodIdn::<PairArgs>::new(
        vir::ViperIdent::new("indexed_indirect_coherence_pair"),
        (
            vir::TYPE_INT,
            vir::TYPE_INT,
            vir::TYPE_PSNAP,
            vir::TYPE_TYVAL,
            vir::TYPE_PSNAP,
        ),
    );
    let v = vcx.mk_local_ex(value);
    let pair_permissions = [permission(from, v), permission(sel, v)];
    let pair_posts = [
        pair_permissions[0],
        pair_permissions[1],
        vcx.mk_eq_expr(state(from, v), state(sel, v)),
        vcx.mk_eq_expr(state(from, v), vcx.mk_old_expr(state(from, v))),
        vcx.mk_eq_expr(state(sel, v), vcx.mk_old_expr(state(sel, v))),
    ];
    let pair_body = vec![
        vcx.mk_unfold_stmt(wrapper(from, v)),
        vcx.mk_unfold_stmt(wrapper(sel, v)),
        vcx.mk_fold_stmt(wrapper(from, v)),
        vcx.mk_fold_stmt(wrapper(sel, v)),
    ];
    let pair_definition = vcx.mk_method(
        pair,
        (start, selected, value, typ, metadata),
        &[],
        vcx.alloc_slice(&pair_permissions),
        vcx.alloc_slice(&pair_posts),
        Some(block(pair_body)),
    );

    let one = MethodIdn::<RangeArgs>::new(
        vir::ViperIdent::new("indexed_indirect_coherence_range"),
        (
            seq_ty,
            vir::TYPE_INT,
            vir::TYPE_INT,
            vir::TYPE_TYVAL,
            vir::TYPE_PSNAP,
        ),
    );
    let bounds = vir::expr! { ((zero) <= (from)) && ((from) <= (length)) };
    let one_pres = [
        bounds,
        range(sel, zero),
        permission(sel, element(sel)),
        resources(from, true),
    ];
    let one_posts = [
        permission(sel, element(sel)),
        resources(from, true),
        vcx.mk_eq_expr(
            state(sel, element(sel)),
            vcx.mk_old_expr(state(sel, element(sel))),
        ),
        frame(from),
        quantify(
            vir::expr! { (([range(x, from)]) && (([element(x)]) == ([element(sel)]))) ==>
            (([state(x, element(x))]) == ([state(sel, element(sel))])) },
        ),
    ];
    let next = (vir::expr! { (from) + ([vcx.mk_uint::<1>()]) }).downcast_ty::<vir::Int>();
    let pair_call = pair(from, sel, element(sel), t, meta).alloc();
    let step = [
        vcx.mk_if_stmt(
            vir::expr! { ([vcx.mk_bin_op_expr(vir::BinOpKind::CmpNe, from, sel).downcast_ty::<vir::Bool>()]) && (([element(from)]) == ([element(sel)])) },
            vcx.alloc_slice(&[pair_call]),
            &[],
        ),
        one(seq, next, sel, t, meta).alloc(),
    ];
    let one_definition = vcx.mk_method(
        one,
        (values, start, selected, typ, metadata),
        &[],
        vcx.alloc_slice(&one_pres),
        vcx.alloc_slice(&one_posts),
        Some(block(vec![vcx.mk_if_stmt(
            vir::expr! { (from) < (length) },
            vcx.alloc_slice(&step),
            &[],
        )])),
    );

    let all = all_method(vcx);
    let all_pres = [bounds, resources(zero, false)];
    let all_coherence = vcx.mk_forall_expr(vcx.alloc_slice(&[k.as_dyn(), j.as_dyn()]),
        vcx.alloc_slice(&[vcx.mk_trigger(&[element(x).as_dyn(), element(y).as_dyn()])]),
        vir::expr! { ([vcx.mk_conj(&[range(x, from), range(y, from), vcx.mk_eq_expr(element(x), element(y))])]) ==>
            (([state(x, element(x))]) == ([state(y, element(y))])) });
    let all_posts = [resources(zero, false), frame(zero), all_coherence];
    // Both recursive methods decrease the nonnegative length - start.
    let all_step = [
        one(seq, zero, from, t, meta).alloc(),
        all(seq, next, t, meta).alloc(),
    ];
    let all_definition = vcx.mk_method(
        all,
        (values, start, typ, metadata),
        &[],
        vcx.alloc_slice(&all_pres),
        vcx.alloc_slice(&all_posts),
        Some(block(vec![vcx.mk_if_stmt(
            vir::expr! { (from) < (length) },
            vcx.alloc_slice(&all_step),
            &[],
        )])),
    );
    vec![pair_definition, one_definition, all_definition]
}

/// A scalar-to-indexed restoration may fold the selected value into one of
/// several equal snapshots. Establish their coherence after restoring all
/// permissions, without assuming distinct values or pointer addresses.
pub(crate) fn restoration_calls<'vir>(
    vcx: &'vir vir::VirCtxt<'vir>,
    lhs: vir::ExprBool<'vir>,
    rhs: vir::ExprBool<'vir>,
) -> Vec<vir::Stmt<'vir>> {
    fn scalar(expr: vir::ExprBool<'_>) -> bool {
        match expr.kind {
            vir::ExprKindData::PredicateApp(app) => app.target == "indirect_resources",
            vir::ExprKindData::BinOp(op) if op.kind == vir::BinOpKind::And => {
                scalar(op.lhs.downcast_ty()) || scalar(op.rhs.downcast_ty())
            }
            _ => false,
        }
    }
    fn variable(expr: vir::ExprDyn<'_>, name: &str) -> bool {
        matches!(expr.kind, vir::ExprKindData::Local(local) if local.name == name)
    }
    fn bounds(expr: vir::ExprBool<'_>, name: &str, lower: &mut bool, upper: &mut bool) -> bool {
        match expr.kind {
            vir::ExprKindData::Const(vir::ConstData::Bool(true)) => true,
            vir::ExprKindData::BinOp(op) => match op.kind {
                vir::BinOpKind::And => {
                    bounds(op.lhs.downcast_ty(), name, lower, upper)
                        && bounds(op.rhs.downcast_ty(), name, lower, upper)
                }
                vir::BinOpKind::CmpLe
                    if matches!(
                        op.lhs.kind,
                        vir::ExprKindData::Const(vir::ConstData::Int(0))
                    ) && variable(op.rhs, name) =>
                {
                    *lower = true;
                    true
                }
                vir::BinOpKind::CmpLt if variable(op.lhs, name) => {
                    *upper = true;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
    fn visit<'vir>(
        vcx: &'vir vir::VirCtxt<'vir>,
        expr: vir::ExprBool<'vir>,
        out: &mut Vec<vir::Stmt<'vir>>,
    ) {
        if let vir::ExprKindData::BinOp(op) = expr.kind
            && op.kind == vir::BinOpKind::And
        {
            visit(vcx, op.lhs.downcast_ty(), out);
            visit(vcx, op.rhs.downcast_ty(), out);
            return;
        }
        let vir::ExprKindData::Forall(q) = expr.kind else {
            return;
        };
        let [index] = q.qvars else { return };
        let vir::ExprKindData::BinOp(body) = q.body.kind else {
            return;
        };
        if body.kind != vir::BinOpKind::Implies {
            return;
        }
        let mut lower = false;
        let mut upper = false;
        if !bounds(body.lhs.downcast_ty(), index.name, &mut lower, &mut upper) || !lower || !upper {
            return;
        }
        let vir::ExprKindData::PredicateApp(app) = body.rhs.kind else {
            return;
        };
        if app.target != "indexed_indirect_resources"
            || app.args.len() != 5
            || !variable(app.args[0], index.name)
        {
            return;
        }
        let vir::ExprKindData::CollectionLiteral(path) = app.args[1].kind else {
            return;
        };
        if !path.values.is_empty() {
            return;
        }
        let vir::ExprKindData::CollectionBinOp(access) = app.args[2].kind else {
            return;
        };
        if access.kind != vir::CollectionBinOpKind::Index || !variable(access.rhs, index.name) {
            return;
        }
        struct Uses<'a>(&'a str, bool);
        impl<'vir> Folder<'vir, (), !> for Uses<'_> {
            fn fold_expr(&mut self, expr: vir::ExprDyn<'vir>) -> Option<vir::ExprDyn<'vir>> {
                self.1 |= variable(expr, self.0);
                expr.super_fold_with(self)
            }
        }
        let mut uses = Uses(index.name, false);
        for value in [access.lhs, app.args[3], app.args[4]] {
            let _ = value.fold_with(&mut uses);
        }
        if uses.1 {
            return;
        }
        out.push(
            all_method(vcx)(
                access.lhs.downcast_ty(),
                vcx.mk_uint::<0>(),
                app.args[3].downcast_ty(),
                app.args[4].downcast_ty(),
            )
            .alloc(),
        );
    }
    let mut out = Vec::new();
    if scalar(lhs) {
        visit(vcx, rhs, &mut out);
    }
    out
}
