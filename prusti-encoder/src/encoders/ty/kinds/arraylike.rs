use crate::encoders::{
    TyUseImpureEnc,
    ty::{
        RustTyDatas,
        data::{ArrayData, TyData},
        impure::{ImpureTyDatas, PredicateBuilder, TyImpureArrayData, TyImpureEnc},
        kinds::opaque::set_opaque,
        pure::{DomainBuilder, PureTyDatas, TyPureArrayData, TyPureEnc},
    },
};
use task_encoder::{EncodeFullError, TaskEncoderDependencies};
use vir::{CastType, HasType};

pub(crate) fn ty_pure<'vir>(
    data: &ArrayData<'vir, RustTyDatas>,
    _deps: &mut TaskEncoderDependencies<'vir, TyPureEnc>,
    builder: &mut DomainBuilder<'vir>,
) -> Result<ArrayData<'vir, PureTyDatas>, EncodeFullError<'vir, TyPureEnc>> {
    let index_access = builder.function(
        "index",
        (builder.self_type(), vir::TYPE_INT),
        vir::TYPE_PSNAP,
    );
    let args = (vir::TYPE_REF, vir::TYPE_INT, builder.params.ty_args());
    let ref_to_index_ref = builder.function("index_ref", args, vir::TYPE_REF);

    // The array-like with the elements of a sequence, for array values built
    // in pure code. Indices outside the sequence are unspecified.
    let seq_ty = builder.vcx.mk_ty_seq(vir::TYPE_PSNAP);
    let from_seq = builder.function("from_seq", seq_ty, builder.self_type());
    let to_seq = builder.function("to_seq", (builder.self_type(), vir::TYPE_INT), seq_ty);
    let zero = builder.vcx.mk_uint::<0>();
    builder.axiom(
        "from_seq",
        vir::expr! {
            forall s: [seq_ty], i: Int :: {[index_access](([from_seq](s)), i)}
                ((([zero]) <= (i)) && ((i) < (|s|)))
                    ==> (([index_access](([from_seq](s)), i)) == (((s)[i]) as PSnap))
        },
    );
    // Slice length lives in the shared reference's metadata, not the slice
    // snapshot. Constrain only nonnegative lengths so arbitrary domain values
    // cannot force a sequence to have a negative length.
    let vcx = builder.vcx;
    let s_decl = vcx.mk_local_decl("s", builder.self_type());
    let n_decl = vcx.mk_local_decl("n", vir::TYPE_INT);
    let i_decl = vcx.mk_local_decl("i", vir::TYPE_INT);
    let s = vcx.mk_local_ex(s_decl);
    let n = vcx.mk_local_ex(n_decl);
    let i = vcx.mk_local_ex(i_decl);
    let seq = to_seq.call()(s, n);
    let len = vcx.mk_collection_len_expr(seq);
    let indexed = vcx.mk_seq_index_expr(seq, i).downcast_ty::<vir::PSnap>();
    let elem = index_access.call()(s, i);
    builder.axiom(
        "to_seq_len",
        vcx.mk_forall_expr(
            vcx.alloc_slice(&[s_decl.as_dyn(), n_decl.as_dyn()]),
            vcx.alloc_slice(&[vcx.mk_trigger(&[seq.as_dyn()])]),
            vir::expr! { (([zero]) <= (n)) ==> ((len) == (n)) },
        ),
    );
    builder.axiom(
        "to_seq_index",
        vcx.mk_forall_expr(
            vcx.alloc_slice(&[s_decl.as_dyn(), n_decl.as_dyn(), i_decl.as_dyn()]),
            vcx.alloc_slice(&[
                vcx.mk_trigger(&[indexed.as_dyn()]),
                vcx.mk_trigger(&[elem.as_dyn(), seq.as_dyn()]),
            ]),
            vir::expr! {
                ((([zero]) <= (i)) && ((i) < (n))) ==> ((indexed) == (elem))
            },
        ),
    );
    Ok(ArrayData::new(
        TyPureArrayData {
            index_access,
            ref_to_index_ref,
            from_seq,
            to_seq,
        },
        data.slice,
    ))
}

pub(crate) fn ty_impure<'vir>(
    task_key: &TyData<'vir, (RustTyDatas, PureTyDatas)>,
    data: &ArrayData<'vir, (RustTyDatas, PureTyDatas)>,
    deps: &mut TaskEncoderDependencies<'vir, TyImpureEnc>,
    builder: &mut PredicateBuilder<'vir>,
) -> Result<ArrayData<'vir, ImpureTyDatas>, EncodeFullError<'vir, TyImpureEnc>> {
    set_opaque(builder);
    let ref_self_decl = builder.ref_self_decl();
    let ref_self = builder.vcx.mk_local_ex(ref_self_decl);
    let index_decl = builder.vcx.mk_local_decl("index", vir::TYPE_INT);
    let index = builder.vcx.mk_local_ex(index_decl);
    let index_predicate = builder
        .inner
        .predicate::<(vir::Ref, vir::Int, vir::ManyTyVal, vir::ManyCSnap)>(
            "index",
            (
                ref_self_decl.ty(),
                index_decl.ty(),
                builder.params.ty_args(),
                builder.params.const_args(),
            ),
            (
                ref_self_decl,
                index_decl,
                builder.params.ty_decls(),
                builder.params.const_decls(),
            ),
            None,
        );

    let ref_to_index_ref = data.1.ref_to_index_ref;
    let index_ref = ref_to_index_ref(ref_self, index, builder.params.ty_exprs());

    let index_frame = builder.inner.function(
        "index_frame",
        (
            ref_self_decl.ty(),
            index_decl.ty(),
            builder.params.ty_args(),
            builder.params.const_args(),
        ),
        task_key.1.snapshot.downcast_ty(),
        (
            ref_self_decl,
            index_decl,
            builder.params.ty_decls(),
            builder.params.const_decls(),
        ),
        &[
            vir::expr! { [index_predicate](ref_self, index, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]]) },
        ],
        &[],
        None,
    );

    let element_ty = data.0.decompose(task_key.0.params);
    let element_ty_out = deps.require_dep::<TyUseImpureEnc>(element_ty)?;
    let element_pred = element_ty_out.ref_to_pred(builder.vcx, index_ref, None);

    let array_snap = vir::expr! { [builder.ref_to_snap](ref_self, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]]) }.downcast_ty();
    let array_index = crate::encoders::ty::indexed::observe_element(
        data.1.index_access.call()(array_snap, index),
        index,
    );
    let method_fold = builder
        .inner
        .method::<(vir::Int, vir::Ref, vir::ManyTyVal, vir::ManyCSnap)>(
            "fold_index",
            (
                vir::TYPE_INT,
                ref_self_decl.ty(),
                builder.params.ty_args(),
                builder.params.const_args(),
            ),
            &[],
            (
                index_decl,
                ref_self_decl,
                builder.params.ty_decls(),
                builder.params.const_decls(),
            ),
            &[
                element_pred,
                vir::expr! { [index_predicate](ref_self, index, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]]) },
            ],
            &[
                vir::expr! { [builder.ref_to_pred](ref_self, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]]) },
                vir::expr! {
                    forall idx: Int :: {[data.1.index_access](array_snap, idx)}
                        ([data.1.index_access](array_snap, idx)) == (
                            ((idx) == (index))
                            ? ([builder.vcx.mk_old_expr(element_ty_out.ref_to_snap(index_ref).downcast_ty())])
                            : (old([data.1.index_access](([index_frame](ref_self, index, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]])), idx)))
                        )
                },
                // Reassembling an unchanged element leaves the whole value
                // unchanged, including its snapshot identity.
                vir::expr! {
                    (([builder.vcx.mk_old_expr(element_ty_out.ref_to_snap(index_ref).downcast_ty::<vir::PSnap>())])
                        == (old([data.1.index_access](([index_frame](ref_self, index, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]])), index))))
                    ==> ((array_snap) == (old([index_frame](ref_self, index, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]]))))
                },
            ],
        );
    let method_unfold = builder
        .inner
        .method::<(vir::Int, vir::Ref, vir::ManyTyVal, vir::ManyCSnap)>(
            "unfold_index",
            (
                vir::TYPE_INT,
                ref_self_decl.ty(),
                builder.params.ty_args(),
                builder.params.const_args(),
            ),
            &[],
            (
                index_decl,
                ref_self_decl,
                builder.params.ty_decls(),
                builder.params.const_decls(),
            ),
            &[
                vir::expr! { [builder.ref_to_pred](ref_self, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]]) },
            ],
            &[
                element_pred,
                vir::expr! { [index_predicate](ref_self, index, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]]) },
                vir::expr! {
                    ([element_ty_out.ref_to_snap(index_ref)])
                    == ([builder.vcx.mk_old_expr(array_index).upcast_ty()])
                },
                vir::expr! {
                    ([index_frame](ref_self, index, [..[builder.params.ty_exprs()]], [..[builder.params.const_exprs()]]))
                    == (old(array_snap))
                }
            ],
        );

    Ok(ArrayData::new(
        TyImpureArrayData {
            ref_to_index_ref,
            index_frame,
            index_predicate,
            method_fold,
            method_unfold,
        },
        data.slice,
    ))
}
