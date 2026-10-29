use super::*;
use crate::{TYPE_BOOL, TYPE_INT};

fn with_vcx(f: impl for<'vir> FnOnce(&'vir VirCtxt<'vir>)) {
    crate::init_vcx(VirCtxt::new_without_tcx());
    crate::with_vcx(f);
}

// Build a binder without invoking its constructor's eager check, so the tests
// exercise the complete surrounding scope in one check.
fn let_node<'vir>(
    vcx: &'vir VirCtxt<'vir>,
    val: ExprDyn<'vir>,
    body: ExprDyn<'vir>,
) -> ExprDyn<'vir> {
    vcx.alloc(ExprGenData::new(vcx.alloc(ExprKindGenData::Let(
        vcx.alloc(LetGenData {
            name: "x",
            val,
            expr: body,
        }),
    ))))
}

#[test]
fn shared_expression_graph() {
    with_vcx(|vcx| {
        let x = vcx.mk_local_decl("x", TYPE_INT);
        let mut e = vcx.mk_local_ex(x).as_dyn();
        // Forty shared nodes represent over a trillion tree occurrences.
        for _ in 0..40 {
            e = vcx.mk_bin_op_expr(BinOpKind::Add, e, e).as_dyn();
        }
        vcx.mk_let_expr(x, vcx.mk_uint::<0>(), e.downcast_ty::<crate::Int>());
    });
}

#[test]
#[should_panic]
fn shared_node_is_checked_in_each_scope() {
    with_vcx(|vcx| {
        let shared = vcx.mk_local_ex(vcx.mk_local_decl("x", TYPE_BOOL)).as_dyn();
        let bound = let_node(vcx, vcx.mk_uint::<0>().as_dyn(), shared);
        let e = vcx.mk_bin_op_expr(BinOpKind::And, shared, bound);
        check_expr_bindings(&mut BindingCheck::default(), e.as_dyn());
    });
}

#[test]
#[should_panic]
fn let_restores_shadowed_binding() {
    with_vcx(|vcx| {
        let boolean = vcx.mk_local_ex(vcx.mk_local_decl("x", TYPE_BOOL)).as_dyn();
        let inner = let_node(vcx, vcx.mk_bool::<true>().as_dyn(), boolean);
        // The second Bool use is outside the inner binding, where x is Int.
        let body = vcx.mk_bin_op_expr(BinOpKind::And, inner, boolean);
        let e = let_node(vcx, vcx.mk_uint::<0>().as_dyn(), body.as_dyn());
        check_expr_bindings(&mut BindingCheck::default(), e.as_dyn());
    });
}

#[test]
#[should_panic]
fn quantifier_restores_shadowed_binding() {
    with_vcx(|vcx| {
        let x = vcx.mk_local_decl("x", TYPE_BOOL);
        let boolean = vcx.mk_local_ex(x);
        let inner = vcx.mk_forall_expr(vcx.alloc_slice(&[x]), &[], boolean);
        let body = vcx.mk_bin_op_expr(BinOpKind::And, inner.as_dyn(), boolean.as_dyn());
        let e = let_node(vcx, vcx.mk_uint::<0>().as_dyn(), body.as_dyn());
        check_expr_bindings(&mut BindingCheck::default(), e.as_dyn());
    });
}

#[test]
fn shared_graph_under_irrelevant_bindings() {
    with_vcx(|vcx| {
        let mut body = vcx.mk_bool::<true>().as_dyn();
        for _ in 0..35 {
            let left = let_node(vcx, vcx.mk_uint::<0>().as_dyn(), body);
            let right = let_node(vcx, vcx.mk_bool::<true>().as_dyn(), body);
            body = vcx.mk_bin_op_expr(BinOpKind::And, left, right).as_dyn();
        }
        check_expr_bindings(&mut BindingCheck::default(), body);
    });
}
