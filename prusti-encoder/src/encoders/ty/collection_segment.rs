//! Inductive ownership of scalar referents stored in an immutable slot sequence.
//! Split and join are checked methods; no address injectivity is assumed.

use std::hash::{Hash, Hasher};
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

use super::{
    RustTyDecomposition, RustTyNormalized, TySpecifics,
    generics::{GArgCaster, GArgsCastEnc, GParams, GenericParams, GenericParamsEnc, TyExprEnc},
    use_impure::TyUseImpureEnc,
    use_pure::TyUsePureEnc,
};

type SegmentArgs = (vir::Seq, vir::Int, vir::Int, vir::ManyTyVal, vir::ManyCSnap);
type SelectionArgs = (
    vir::Seq,
    vir::Int,
    vir::Int,
    vir::Int,
    vir::ManyTyVal,
    vir::ManyCSnap,
);

pub(crate) struct CollectionSegmentEnc;

#[derive(Clone)]
pub(crate) struct CollectionSegment<'vir> {
    predicate: vir::PredicateIdn<'vir, SegmentArgs>,
    states: vir::FunctionIdn<'vir, SegmentArgs, vir::Seq>,
    pub split: vir::MethodIdn<'vir, SelectionArgs>,
    pub join: vir::MethodIdn<'vir, SelectionArgs>,
    pub params: GenericParams<'vir>,
    reference: super::use_pure::TyUsePureMutRef<'vir>,
    caster: GArgCaster<'vir, crate::encoders::Pure>,
    pub referent_type: vir::ExprTyVal<'vir>,
}

impl<'vir> CollectionSegment<'vir> {
    pub fn address(
        &self,
        sequence: vir::ExprSeq<'vir>,
        index: vir::ExprInt<'vir>,
    ) -> vir::ExprRef<'vir> {
        vir::with_vcx(|vcx| {
            self.reference.deref_access(
                self.caster
                    .cast_to_caller_ctx(
                        vcx.mk_seq_index_expr(sequence, index)
                            .downcast_ty::<vir::Snap>(),
                    )
                    .downcast_ty(),
            )
        })
    }
    pub fn whole(&self, sequence: vir::ExprSeq<'vir>) -> vir::ExprBool<'vir> {
        vir::with_vcx(|vcx| {
            vcx.mk_predicate_app_expr((self.predicate)(
                sequence,
                vcx.mk_int::<0>(),
                vcx.mk_collection_len_expr(sequence),
                self.params.ty_exprs(),
                self.params.const_exprs(),
            )(None))
        })
    }

    pub fn observe(&self, sequence: vir::ExprSeq<'vir>) -> vir::ExprSeq<'vir> {
        vir::with_vcx(|vcx| {
            self.states.call()(
                sequence,
                vcx.mk_int::<0>(),
                vcx.mk_collection_len_expr(sequence),
                self.params.ty_exprs(),
                self.params.const_exprs(),
            )
        })
    }

    pub fn observe_lazy<Curr>(
        &self,
        sequence: vir::ExprGen<'vir, Curr, vir::ExprKind<'vir>, vir::Seq>,
    ) -> vir::ExprGen<'vir, Curr, vir::ExprKind<'vir>, vir::Seq> {
        vir::with_vcx(|vcx| {
            self.states.call()(
                sequence,
                vcx.mk_int::<0>().lift(),
                vcx.mk_collection_len_expr(sequence),
                &self
                    .params
                    .ty_exprs()
                    .iter()
                    .map(|e| e.lift())
                    .collect::<Vec<_>>(),
                &self
                    .params
                    .const_exprs()
                    .iter()
                    .map(|e| e.lift())
                    .collect::<Vec<_>>(),
            )
        })
    }
}

#[derive(Clone)]
pub(crate) struct SegmentDefinitions<'vir> {
    predicate: vir::Predicate<'vir>,
    states: vir::Function<'vir>,
    split: vir::Method<'vir>,
    join: vir::Method<'vir>,
    abstract_snapshot: &'vir str,
}

impl TaskEncoder for CollectionSegmentEnc {
    task_encoder::encoder_cache!(CollectionSegmentEnc);
    const ENCODER_NAME: &'static str = "checked referent segment";
    type TaskDescription<'vir> = RustTyDecomposition<'vir>;
    type OutputFullDependency<'vir> = CollectionSegment<'vir>;
    type OutputFullLocal<'vir> = SegmentDefinitions<'vir>;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        // The resource representation depends on the referent's concrete
        // scalar type, not the Rust lifetime or specification-closure context.
        // Region ownership is selected by the caller before constructing this
        // predicate; retaining closure-local regions here creates incompatible
        // names for the same physical resource at entry and observation sites.
        vir::with_vcx(|vcx| {
            let referent = task
                .args
                .context()
                .normalize(task.args.args()[1].expect_ty());
            assert!(referent.is_primitive(), "scalar referent segment required");
            let reference = prusti_rustc_interface::middle::ty::Ty::new_mut_ref(
                vcx.tcx(),
                vcx.tcx().lifetimes.re_erased,
                referent,
            );
            RustTyDecomposition::from_ty(reference, GParams::empty())
        })
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(*task, ())?;
        let ty = *task;
        let pure = deps.require_dep::<TyUsePureEnc>(ty)?;
        let TySpecifics::MutRef((data, reference)) = ty.ty.zip(pure).specifics else {
            unreachable!("referent segments require mutable-reference elements")
        };
        let physical = deps.require_dep::<TyUseImpureEnc>(
            data.referent.decompose_context(ty.ty.params, ty.args),
        )?;
        let referent_type =
            deps.require_dep::<TyExprEnc>(data.referent.decompose_normalize(ty.args))?;
        let caster =
            deps.require_dep::<GArgsCastEnc<crate::encoders::Pure>>(Some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: ty,
            }))?;
        let params = deps.require_dep::<GenericParamsEnc>(ty.args.context())?;
        vir::with_vcx(|vcx| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            format!("{task:?}").hash(&mut hash);
            let suffix = hash.finish();
            let seq_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
            let sequence = vcx.mk_local_decl("elements", seq_ty);
            let lower = vcx.mk_local_decl("lower", vir::TYPE_INT);
            let upper = vcx.mk_local_decl("upper", vir::TYPE_INT);
            let selected = vcx.mk_local_decl("selected", vir::TYPE_INT);
            let e = vcx.mk_local_ex(sequence);
            let lo = vcx.mk_local_ex(lower);
            let hi = vcx.mk_local_ex(upper);
            let i = vcx.mk_local_ex(selected);
            let zero = vcx.mk_int::<0>();
            let one = vcx.mk_int::<1>();
            let plus_one = |x| {
                vcx.mk_bin_op_expr(vir::BinOpKind::Add, x, one)
                    .downcast_ty::<vir::Int>()
            };
            let extent = vcx
                .mk_bin_op_expr(vir::BinOpKind::Sub, hi, lo)
                .downcast_ty::<vir::Int>();
            let bounds = vcx.mk_conj(&[
                vir::expr! { (zero) <= (lo) },
                vir::expr! { (lo) <= (hi) },
                vir::expr! { (hi) <= (|e|) },
            ]);
            let arity = (
                seq_ty,
                vir::TYPE_INT,
                vir::TYPE_INT,
                params.ty_args(),
                params.const_args(),
            );
            let predicate = vir::PredicateIdn::new(
                vir::vir_format_identifier!(vcx, "referent_segment_{suffix:x}"),
                arity,
            );
            let states = vir::FunctionIdn::new(
                vir::vir_format_identifier!(vcx, "referent_states_{suffix:x}"),
                arity,
                seq_ty,
            );
            let segment = |a, b| predicate(e, a, b, params.ty_exprs(), params.const_exprs())(None);
            let access = |a, b| vcx.mk_predicate_app_expr(segment(a, b));
            let values = |a, b| states.call()(e, a, b, params.ty_exprs(), params.const_exprs());
            let address = |index| {
                let element = caster
                    .cast_to_caller_ctx(vcx.mk_seq_index_expr(e, index).downcast_ty::<vir::Snap>());
                reference.deref_access(element.downcast_ty())
            };
            let loan = |index| physical.ref_to_pred(vcx, address(index), None);
            let read = |index| {
                physical
                    .ref_to_snap(address(index))
                    .downcast_ty::<vir::PSnap>()
            };
            let vir::ExprKindData::FuncApp(snapshot_call) = read(lo).kind else {
                unreachable!()
            };
            let pred_body = vcx.mk_conj(&[
                bounds,
                vir::expr! { ((lo) < (hi)) ==> (([loan(lo)]) && ([access(plus_one(lo), hi)])) },
            ]);
            let predicate_def = vcx.mk_predicate(
                predicate,
                (
                    sequence,
                    lower,
                    upper,
                    params.ty_decls(),
                    params.const_decls(),
                ),
                Some(pred_body),
            );
            let result = vcx.mk_result(seq_ty);
            let singleton =
                |value| vcx.mk_seq_literal_expr(vcx.alloc_slice(&[value]), vir::TYPE_PSNAP);
            let states_def = vcx.mk_function(
                states,
                (
                    sequence,
                    lower,
                    upper,
                    params.ty_decls(),
                    params.const_decls(),
                ),
                vcx.alloc_slice(&[bounds, access(lo, hi)]),
                vcx.alloc_slice(&[vcx.mk_eq_expr(vcx.mk_collection_len_expr(result), extent)]),
                Some(vcx.alloc(vir::DecreasesGenData::Tuple(
                    vcx.alloc_slice(&[extent.as_dyn()]),
                    None,
                ))),
                Some(vcx.mk_unfolding_expr(
                    segment(lo, hi),
                    vcx.mk_ternary_expr(
                        vcx.mk_eq_expr(lo, hi),
                        vcx.mk_seq_literal_expr(&[], vir::TYPE_PSNAP),
                        vcx.mk_seq_concat_expr(singleton(read(lo)), values(plus_one(lo), hi)),
                    ),
                )),
            );
            let selection_arity = (
                seq_ty,
                vir::TYPE_INT,
                vir::TYPE_INT,
                vir::TYPE_INT,
                params.ty_args(),
                params.const_args(),
            );
            let split = vir::MethodIdn::new(
                vir::vir_format_identifier!(vcx, "split_referents_{suffix:x}"),
                selection_arity,
            );
            let join = vir::MethodIdn::new(
                vir::vir_format_identifier!(vcx, "join_referents_{suffix:x}"),
                selection_arity,
            );
            let selection_bounds = vcx.mk_conj(&[
                vir::expr! { (zero) <= (lo) },
                vir::expr! { (lo) <= (i) },
                vir::expr! { (i) < (hi) },
                vir::expr! { (hi) <= (|e|) },
            ]);
            let pieces = vcx.mk_conj(&[access(lo, i), loan(i), access(plus_one(i), hi)]);
            let concatenated = vcx.mk_seq_concat_expr(
                values(lo, i),
                vcx.mk_seq_concat_expr(singleton(read(i)), values(plus_one(i), hi)),
            );
            let measure = vcx
                .mk_bin_op_expr(vir::BinOpKind::Sub, i, lo)
                .downcast_ty::<vir::Int>();
            let decreases = vcx.alloc(vir::DecreasesGenData::Tuple(
                vcx.alloc_slice(&[measure.as_dyn()]),
                None,
            ));
            let recurse = |method: vir::MethodIdn<'vir, SelectionArgs>| {
                method(
                    e,
                    plus_one(lo),
                    i,
                    hi,
                    params.ty_exprs(),
                    params.const_exprs(),
                )
                .alloc()
            };
            let body = |stmts: &[vir::Stmt<'vir>]| {
                vcx.alloc_slice(&[vcx.mk_cfg_block(
                    &vir::CfgBlockLabelData::End,
                    &[],
                    vcx.alloc_slice(stmts),
                    &vir::TerminatorStmtData::Exit,
                )])
            };
            let split_def = vcx.method_with_decreases(
                vcx.mk_method(
                    split,
                    (
                        sequence,
                        lower,
                        selected,
                        upper,
                        params.ty_decls(),
                        params.const_decls(),
                    ),
                    &[],
                    vcx.alloc_slice(&[selection_bounds, access(lo, hi)]),
                    vcx.alloc_slice(&[
                        pieces,
                        vcx.mk_eq_expr(vcx.mk_old_expr(values(lo, hi)), concatenated),
                    ]),
                    Some(body(&[
                        vcx.mk_unfold_stmt(segment(lo, hi)),
                        vcx.mk_if_stmt(
                            vir::expr! { (lo) < (i) },
                            vcx.alloc_slice(&[recurse(split), vcx.mk_fold_stmt(segment(lo, i))]),
                            vcx.alloc_slice(&[vcx.mk_fold_stmt(segment(lo, lo))]),
                        ),
                    ])),
                ),
                decreases,
            );
            let join_def = vcx.method_with_decreases(
                vcx.mk_method(
                    join,
                    (
                        sequence,
                        lower,
                        selected,
                        upper,
                        params.ty_decls(),
                        params.const_decls(),
                    ),
                    &[],
                    vcx.alloc_slice(&[selection_bounds, pieces]),
                    vcx.alloc_slice(&[
                        access(lo, hi),
                        vcx.mk_eq_expr(values(lo, hi), vcx.mk_old_expr(concatenated)),
                    ]),
                    Some(body(&[
                        vcx.mk_unfold_stmt(segment(lo, i)),
                        vcx.mk_if_stmt(
                            vir::expr! { (lo) < (i) },
                            vcx.alloc_slice(&[recurse(join)]),
                            &[],
                        ),
                        vcx.mk_fold_stmt(segment(lo, hi)),
                    ])),
                ),
                decreases,
            );
            Ok((
                SegmentDefinitions {
                    predicate: predicate_def,
                    states: states_def,
                    split: split_def,
                    join: join_def,
                    abstract_snapshot: snapshot_call.target,
                },
                CollectionSegment {
                    predicate,
                    states,
                    split,
                    join,
                    params,
                    reference: reference.clone(),
                    caster,
                    referent_type,
                },
            ))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for output in Self::all_outputs_local_no_errors(program) {
            program.require_total_abstract_function(output.abstract_snapshot);
            program.add_predicate(output.predicate);
            program.add_function(output.states);
            program.add_method(output.split);
            program.add_method(output.join);
        }
    }
}
