//! Permission-checked shallow snapshots for indexed mutable-reference chains.
//!
//! Capturing the whole sequence before packaging a wand keeps its resource
//! addresses independent of later writes to the parent references. This does
//! not transfer permissions or observe the values behind the captured pointers.
use super::{
    RustTyDecomposition, RustTyNormalized, TySpecifics,
    generics::{GArgs, GArgsCastEnc, GParams, GenericParams, GenericParamsEnc},
    indexed::{IndexedResources, quantify_resources},
    use_impure::TyUseImpureEnc,
    use_pure::TyUsePureEnc,
};
use prusti_rustc_interface::middle::ty;
use std::hash::{Hash, Hasher};
use task_encoder::{EncodeFullResult, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

type Arguments = (vir::Seq, vir::Int, vir::ManyTyVal, vir::ManyCSnap);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexedSnapshotTask<'vir> {
    pub reference: RustTyDecomposition<'vir>,
    pub path: Vec<usize>,
}

pub(crate) struct IndexedSnapshotEnc;

#[derive(Clone)]
pub(crate) struct IndexedSnapshot<'vir> {
    function: vir::FunctionIdn<'vir, Arguments, vir::Seq>,
    params: GenericParams<'vir>,
}

impl<'vir> IndexedSnapshot<'vir> {
    pub fn observe(&self, sequence: vir::ExprSeq<'vir>) -> vir::ExprSeq<'vir> {
        vir::with_vcx(|vcx| {
            self.function.call()(
                sequence,
                vcx.mk_uint::<0>(),
                self.params.ty_exprs(),
                self.params.const_exprs(),
            )
        })
    }
}

impl TaskEncoder for IndexedSnapshotEnc {
    task_encoder::encoder_cache!(IndexedSnapshotEnc);
    const ENCODER_NAME: &'static str = "indexed shallow snapshot encoder";
    type TaskDescription<'vir> = IndexedSnapshotTask<'vir>;
    type OutputFullDependency<'vir> = IndexedSnapshot<'vir>;
    type OutputFullLocal<'vir> = Vec<vir::Function<'vir>>;

    fn task_to_key<'vir>(task: &Self::TaskDescription<'vir>) -> Self::TaskKey<'vir> {
        let mut key = task.clone();
        vir::with_vcx(|vcx| {
            // These observers read physical shallow values. Their predicates
            // and result contain no Rust lifetime identity; the indexed path
            // still distinguishes the logical ownership position. Concrete
            // types must share an observer across caller and callee contexts.
            let args = vcx
                .tcx()
                .erase_and_anonymize_regions(vcx.tcx().mk_args(task.reference.args.args()));
            let ground = args
                .iter()
                .flat_map(|arg| arg.walk())
                .all(|arg| match arg.kind() {
                    ty::GenericArgKind::Lifetime(_) => true,
                    ty::GenericArgKind::Const(value) => {
                        matches!(value.kind(), ty::ConstKind::Value(_))
                    }
                    ty::GenericArgKind::Type(value) => matches!(
                        value.kind(),
                        ty::TyKind::Bool
                            | ty::TyKind::Char
                            | ty::TyKind::Int(_)
                            | ty::TyKind::Uint(_)
                            | ty::TyKind::Float(_)
                            | ty::TyKind::Str
                            | ty::TyKind::Never
                            | ty::TyKind::Tuple(_)
                            | ty::TyKind::Array(..)
                            | ty::TyKind::Slice(_)
                            | ty::TyKind::Ref(..)
                            | ty::TyKind::RawPtr(..)
                            | ty::TyKind::Adt(..)
                    ),
                });
            if ground {
                key.reference.args = GArgs::new(GParams::empty(), args);
            }
        });
        key
    }

    fn do_encode_full<'vir>(
        task: &Self::TaskKey<'vir>,
        deps: &mut TaskEncoderDependencies<'vir, Self>,
    ) -> EncodeFullResult<'vir, Self> {
        deps.emit_output_ref(task.clone(), ())?;
        let ty = task.reference;
        let pure = deps.require_dep::<TyUsePureEnc>(ty)?;
        let TySpecifics::MutRef((data, reference)) = ty.ty.zip(pure).specifics else {
            unreachable!("indexed snapshot tasks describe mutable references")
        };
        let referent = data.referent.decompose_context(ty.ty.params, ty.args);
        let physical = deps.require_dep::<TyUseImpureEnc>(referent)?;
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
            let path = vcx.alloc_slice(&task.path);
            let seq_ty = vcx.mk_ty_seq(vir::TYPE_PSNAP);
            let sequence = vcx.mk_local_decl("snapshot_elements", seq_ty);
            let start = vcx.mk_local_decl("snapshot_start", vir::TYPE_INT);
            let seq = vcx.mk_local_ex(sequence);
            let i = vcx.mk_local_ex(start);
            let length = vcx.mk_collection_len_expr(seq);
            let zero = vcx.mk_uint::<0>();
            let address = |index| {
                let element = caster.cast_to_caller_ctx(
                    vcx.mk_seq_index_expr(seq, index).downcast_ty::<vir::Snap>(),
                );
                reference.deref_access(element.downcast_ty())
            };
            let indexed = IndexedResources::new(deps, physical.ref_to_pred(vcx, address(i), None))?;
            let arity = (seq_ty, vir::TYPE_INT, params.ty_args(), params.const_args());
            let element = vir::FunctionIdn::<Arguments, vir::PSnap>::new(
                vir::vir_format_identifier!(vcx, "indexed_shallow_element_{suffix:x}"),
                arity,
                vir::TYPE_PSNAP,
            );
            let function = vir::FunctionIdn::<Arguments, vir::Seq>::new(
                vir::vir_format_identifier!(vcx, "indexed_shallow_sequence_{suffix:x}"),
                arity,
                seq_ty,
            );
            let head = indexed
                .read_snapshot(
                    physical.ref_to_pred_app(address(i), None),
                    physical.ref_to_snap(address(i)),
                    i,
                    false,
                    path,
                )
                .downcast_ty::<vir::PSnap>();
            let element_definition = vcx.mk_function(
                element,
                (sequence, start, params.ty_decls(), params.const_decls()),
                vcx.alloc_slice(&[
                    vir::expr! { ((zero) <= (i)) && ((i) < (length)) },
                    indexed.wrap(physical.ref_to_pred(vcx, address(i), None), i, path),
                ]),
                &[],
                None,
                Some(head),
            );
            let binder = vcx.mk_local_decl("snapshot_index", vir::TYPE_INT);
            let j = vcx.mk_local_ex(binder);
            let variables = vcx.alloc_slice(&[binder.as_dyn()]);
            let resources = quantify_resources(
                vcx,
                variables,
                vcx.alloc_slice(&[vcx.mk_trigger(&[vcx.mk_seq_index_expr(seq, j)])]),
                vir::expr! { ((i) <= (j)) && ((j) < (length)) },
                indexed.wrap(physical.ref_to_pred(vcx, address(j), None), j, path),
            );
            let observe =
                |index| element.call()(seq, index, params.ty_exprs(), params.const_exprs());
            let next = (vir::expr! { (i) + ([vcx.mk_uint::<1>()]) }).downcast_ty();
            let body = vcx.mk_ternary_expr(
                vcx.mk_eq_expr(i, length),
                vcx.mk_seq_literal_expr(&[], vir::TYPE_PSNAP),
                vcx.mk_seq_concat_expr(
                    vcx.mk_seq_literal_expr(vcx.alloc_slice(&[observe(i)]), vir::TYPE_PSNAP),
                    function.call()(seq, next, params.ty_exprs(), params.const_exprs()),
                ),
            );
            let result = vcx.mk_result(seq_ty);
            let extent = vcx
                .mk_bin_op_expr(vir::BinOpKind::Sub, length, i)
                .downcast_ty::<vir::Int>();
            let selected = vcx.mk_seq_index_expr(result, j).downcast_ty::<vir::PSnap>();
            let absolute = vcx
                .mk_bin_op_expr(vir::BinOpKind::Add, j, i)
                .downcast_ty::<vir::Int>();
            let elements = vcx.mk_forall_expr(variables,
                vcx.alloc_slice(&[vcx.mk_trigger(&[selected.as_dyn()])]),
                vir::expr! { (((zero) <= (j)) && ((j) < (extent))) ==> ((selected) == ([observe(absolute)])) },
            );
            // State the same relation from the input index as well. Restoration
            // scripts refer to the original array, whereas the wand uses the
            // captured sequence. Either lookup must instantiate their equality.
            // A separate absolute-index binder avoids arithmetic in a trigger.
            let relative = vcx
                .mk_bin_op_expr(vir::BinOpKind::Sub, j, i)
                .downcast_ty::<vir::Int>();
            let input_elements = vcx.mk_forall_expr(variables,
                vcx.alloc_slice(&[vcx.mk_trigger(&[vcx.mk_seq_index_expr(seq, j)])]),
                vir::expr! { (((i) <= (j)) && ((j) < (length))) ==>
                    (([vcx.mk_seq_index_expr(result, relative).downcast_ty::<vir::PSnap>()]) == ([observe(j)])) },
            );
            let definition = vcx.opaque_function(vcx.mk_function(
                function,
                (sequence, start, params.ty_decls(), params.const_decls()),
                vcx.alloc_slice(&[
                    vir::expr! { ((zero) <= (i)) && ((i) <= (length)) },
                    resources,
                ]),
                vcx.alloc_slice(&[
                    vcx.mk_eq_expr(vcx.mk_collection_len_expr(result), extent),
                    elements,
                    input_elements,
                ]),
                None,
                Some(body),
            ));
            Ok((
                vec![element_definition, definition],
                IndexedSnapshot { function, params },
            ))
        })
    }

    fn emit_outputs<'vir>(program: &mut task_encoder::Program<'vir>) {
        for functions in Self::all_outputs_local_no_errors(program) {
            for function in functions {
                program.add_function(function);
            }
        }
    }
}
