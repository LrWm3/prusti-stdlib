//! Standard integer well-founded order for directly constructed Silver ASTs.
//!
//! Text frontends import `decreases/declaration.vpr` and `decreases/int.vpr`.
//! Preserve those same declarations and implications in VIR, including in its
//! serialization and hash, instead of relying on a parser to supply them.

use crate::*;

pub(crate) fn integer_termination_domains<'vir, 'tcx, Curr: 'vir, Next: 'vir>(
    vcx: &'vir VirCtxt<'tcx>,
) -> [DomainGen<'vir, Curr, Next>; 2] {
    let parameter = vcx.alloc(DomainParamData {
        name: "T",
        index: 0,
    });
    let ty = vcx.alloc(TypeData::<Dyn>::new(TypeKind::DomainTypeParam(*parameter)));
    let decreasing =
        FunctionIdn::<(Dyn, Dyn), Bool>::new(ViperIdent::new("decreasing"), (ty, ty), TYPE_BOOL);
    let bounded = FunctionIdn::<Dyn, Bool>::new(ViperIdent::new("bounded"), ty, TYPE_BOOL);
    let declaration = vcx.alloc(DomainGenData {
        name: "WellFoundedOrder",
        typarams: vcx.alloc_slice(&[parameter]),
        axioms: &[],
        functions: vcx.alloc_slice(&[
            vcx.mk_domain_function(decreasing, false, None),
            vcx.mk_domain_function(bounded, false, None),
        ]),
        interpretation: None,
    });
    let left = vcx.mk_local_decl("int1", TYPE_INT);
    let right = vcx.mk_local_decl("int2", TYPE_INT);
    let x: ExprGenInt<'vir, Curr, Next> = vcx.mk_local_ex(left);
    let y = vcx.mk_local_ex(right);
    let substitutions = vcx.alloc_slice(&[TYPE_INT.as_dyn()]);
    let dec = vcx.mk_func_app(
        "decreasing",
        vcx.alloc_slice(&[x.as_dyn(), y.as_dyn()]),
        TYPE_BOOL,
        substitutions,
    );
    let bound = vcx.mk_func_app(
        "bounded",
        vcx.alloc_slice(&[x.as_dyn()]),
        TYPE_BOOL,
        substitutions,
    );
    let less = vcx
        .mk_bin_op_expr(BinOpKind::CmpLt, x, y)
        .downcast_ty::<Bool>();
    let nonnegative = vcx
        .mk_bin_op_expr(
            BinOpKind::CmpGe,
            x,
            vcx.mk_const_expr(ConstData::Int(0)).downcast_ty::<Int>(),
        )
        .downcast_ty::<Bool>();
    let integer = vcx.alloc(DomainGenData {
        name: "IntWellFoundedOrder",
        typarams: &[],
        functions: &[],
        axioms: vcx.alloc_slice(&[
            vcx.alloc(DomainAxiomGenData {
                name: "integer_ax_dec",
                expr: vcx.mk_forall_expr(
                    vcx.alloc_slice(&[left, right]),
                    vcx.alloc_slice(&[vcx.mk_trigger(vcx.alloc_slice(&[dec]))]),
                    vcx.mk_bin_op_expr(BinOpKind::Implies, less, dec)
                        .downcast_ty(),
                ),
            }),
            vcx.alloc(DomainAxiomGenData {
                name: "integer_ax_bound",
                expr: vcx.mk_forall_expr(
                    vcx.alloc_slice(&[left]),
                    vcx.alloc_slice(&[vcx.mk_trigger(vcx.alloc_slice(&[bound]))]),
                    vcx.mk_bin_op_expr(BinOpKind::Implies, nonnegative, bound)
                        .downcast_ty(),
                ),
            }),
        ]),
        interpretation: None,
    });
    [declaration, integer]
}
