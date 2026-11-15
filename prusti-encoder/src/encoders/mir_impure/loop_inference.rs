//! First inference experiment: a single shared-slice traversal and usize count.
//! Candidate facts never enter as assumptions: the ordinary loop encoder checks
//! initialization, preservation, permissions and all machine-arithmetic checks.
mod affine;
use super::*;
use crate::encoders::FunctionCallEnc;

pub(super) struct CountShape<'vir> {
    source: mir::Local,
    counter: mir::Local,
    iterator: mir::Local,
    remaining: DefId,
    delta: i64,
    selected_contracts: Vec<DefId>,
    iterator_ty: ty::Ty<'vir>,
}
impl CountShape<'_> {
    pub(super) fn inputs(&self) -> [mir::Local; 3] {
        [self.source, self.counter, self.iterator]
    }
}
fn direct(operand: &mir::Operand<'_>) -> Option<mir::Local> {
    let p = operand.place()?;
    p.projection.is_empty().then_some(p.local)
}
fn literal(operand: &mir::Operand<'_>) -> Option<i64> {
    let mir::Operand::Constant(c) = operand else {
        return None;
    };
    let mir::Const::Val(mir::ConstValue::Scalar(mir::interpret::Scalar::Int(v)), _) = c.const_
    else {
        return None;
    };
    i64::try_from(v.to_bits(v.size())).ok()
}
impl<'vir, E: TaskEncoder> ImpureEncVisitor<'vir, '_, E> {
    /// Extract one supported loop, retaining the selected conditional contract
    /// identities. No finite behavior is inferred from Iterator alone.
    pub(super) fn count_shape(&self, id: LoopId) -> Option<CountShape<'vir>> {
        if !config::infer_loop_invariants() || self.loop_analysis().all_loops().count() != 1 {
            return None;
        }
        let tcx = self.vcx.tcx();
        let head = self.loop_analysis()[id];
        let dom = self.body.basic_blocks.dominators();
        let inside = |b| self.loop_analysis().in_loop(b, id);
        let mut next = None;
        let mut switches = Vec::new();
        for (bb, data) in self
            .body
            .basic_blocks
            .iter_enumerated()
            .filter(|(b, _)| inside(*b))
        {
            match &data.terminator().kind {
                mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    let ty::FnDef(..) = *func.ty(self.body, tcx).kind() else {
                        return None;
                    };
                    let (callee, gargs, _) = self.get_call_data(func);
                    if tcx.item_name(callee).as_str() != "next"
                        || tcx.trait_of_assoc(callee) != tcx.lang_items().iterator_trait()
                        || next.is_some()
                    {
                        return None;
                    }
                    let argument = direct(&args.first()?.node)?;
                    let iterator = data.statements.iter().find_map(|s| {
                        if let mir::StatementKind::Assign(box (to, mir::Rvalue::Ref(_, _, from))) =
                            &s.kind
                            && to.as_local() == Some(argument)
                        {
                            from.as_local()
                        } else {
                            None
                        }
                    })?;
                    let iterator_ty = self.local_decls[iterator].ty;
                    let ty::TyKind::Adt(adt, _) = iterator_ty.kind() else {
                        return None;
                    };
                    if tcx.crate_name(adt.did().krate).as_str() != "core"
                        || tcx.def_path(adt.did()).to_string_no_crate_verbose()
                            != "::slice::iter::Iter"
                    {
                        return None;
                    }
                    let selected = self.conditional_specs_for_call(callee, gargs, false);
                    if selected.is_empty() {
                        return None;
                    }
                    // Resolve the observation from the selected contract's MIR,
                    // rather than inventing a model for the receiver type.
                    let mut remaining = None;
                    for post in &selected {
                        let body =
                            self.vcx
                                .body_mut()
                                .get_spec_body(*post, gargs, Some(self.def_id));
                        for block in body.basic_blocks.iter() {
                            if let mir::TerminatorKind::Call { func, .. } = &block.terminator().kind
                                && let ty::FnDef(did, _) = *func.ty(&body.local_decls, tcx).kind()
                                && tcx.item_name(did).as_str() == "remaining"
                                && let Some(trait_id) = tcx.trait_of_assoc(did)
                                && tcx.crate_name(trait_id.krate).as_str() == "prusti_contracts"
                                && tcx.def_path(trait_id).to_string_no_crate_verbose()
                                    == "::iter_model::ModeledIterator"
                            {
                                if remaining.is_some_and(|r| r != did) {
                                    return None;
                                }
                                remaining = Some(did);
                            }
                        }
                    }
                    next = Some((
                        bb,
                        iterator,
                        iterator_ty,
                        destination.as_local()?,
                        remaining?,
                        selected,
                    ));
                }
                mir::TerminatorKind::SwitchInt { discr, targets } => {
                    switches.push((bb, discr, targets))
                }
                mir::TerminatorKind::Goto { .. }
                | mir::TerminatorKind::FalseEdge { .. }
                | mir::TerminatorKind::FalseUnwind { .. }
                | mir::TerminatorKind::Assert { .. }
                | mir::TerminatorKind::Unreachable => {}
                _ => return None,
            }
        }
        let (next_bb, iterator, iterator_ty, result, remaining, selected_contracts) = next?;
        // Only the Some/None split is supported; early-break and additional-next
        // shapes fall back to ordinary verification without an invented summary.
        if switches.len() != 1 {
            return None;
        }
        let (switch_bb, discr, targets) = switches[0];
        let disc = direct(discr)?;
        if !self.body[switch_bb].statements.iter().any(|s| {
            matches!(&s.kind,
            mir::StatementKind::Assign(box (to,mir::Rvalue::Discriminant(from)))
                if to.as_local()==Some(disc) && from.as_local()==Some(result))
        }) {
            return None;
        }
        let some = targets.iter().find_map(|(v, b)| (v == 1).then_some(b))?;
        let none = targets.iter().find_map(|(v, b)| (v == 0).then_some(b))?;
        if inside(none) || !dom.dominates(next_bb, switch_bb) {
            return None;
        }
        let backedges = self
            .body
            .basic_blocks
            .iter_enumerated()
            .filter(|(b, d)| inside(*b) && d.terminator().successors().any(|s| s == head))
            .map(|(b, _)| b)
            .collect::<Vec<_>>();
        if backedges.len() != 1 || !dom.dominates(some, backedges[0]) {
            return None;
        }
        // Read the counter from the return flow; source names have no role.
        let counters = self
            .body
            .basic_blocks
            .iter_enumerated()
            .filter(|(b, _)| !inside(*b))
            .flat_map(|(_, d)| d.statements.iter())
            .filter_map(|s| {
                if let mir::StatementKind::Assign(box (to, mir::Rvalue::Use(from, _))) = &s.kind
                    && to.as_local() == Some(mir::RETURN_PLACE)
                {
                    direct(from)
                } else {
                    None
                }
            })
            .collect::<FxHashSet<_>>();
        if counters.len() != 1 {
            return None;
        }
        let counter = *counters.iter().next()?;
        if self.local_decls[counter].ty != tcx.types.usize {
            return None;
        }
        let mut initial = false;
        let mut update = None;
        let mut writes = Vec::new();
        let mut source = None;
        for (bb, data) in self.body.basic_blocks.iter_enumerated() {
            if !inside(bb)
                && dom.dominates(bb, head)
                && let mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } = &data.terminator().kind
                && destination.as_local() == Some(iterator)
            {
                let (constructor, _, _) = self.get_call_data(func);
                let implementation = tcx.impl_of_assoc(constructor)?;
                if tcx.crate_name(constructor.krate).as_str() != "core"
                    || tcx.item_name(constructor).as_str() != "iter"
                    || !tcx
                        .type_of(implementation)
                        .instantiate_identity()
                        .skip_normalization()
                        .is_slice()
                {
                    return None;
                }
                let receiver = direct(&args.first()?.node)?;
                source = data.statements.iter().find_map(|s| {
                    if let mir::StatementKind::Assign(box (
                        to,
                        mir::Rvalue::Ref(_, mir::BorrowKind::Shared, from),
                    )) = &s.kind
                        && to.as_local() == Some(receiver)
                        && matches!(from.projection.as_ref(), [mir::ProjectionElem::Deref])
                    {
                        Some(from.local)
                    } else {
                        None
                    }
                });
            }
            for statement in &data.statements {
                let mir::StatementKind::Assign(box (to, value)) = &statement.kind else {
                    continue;
                };
                if to.as_local() == Some(counter) {
                    if inside(bb) {
                        writes.push((bb, value));
                    } else if dom.dominates(bb, head)
                        && matches!(value,mir::Rvalue::Use(op,_) if literal(op)==Some(0))
                    {
                        initial = true;
                    } else {
                        return None;
                    }
                }
                if inside(bb)
                    && let mir::Rvalue::BinaryOp(mir::BinOp::AddWithOverflow, box (left, right)) =
                        value
                    && direct(left) == Some(counter)
                {
                    if update.is_some()
                        || !dom.dominates(some, bb)
                        || !dom.dominates(bb, backedges[0])
                    {
                        return None;
                    }
                    update = Some((to.as_local()?, literal(right)?));
                }
                // Reject escaping/mutating aliases of the counter.
                if let mir::Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, p) = value
                    && p.local == counter
                {
                    return None;
                }
            }
        }
        if !initial {
            return None;
        }
        let delta = if writes.is_empty() {
            0
        } else {
            if writes.len() != 1 {
                return None;
            }
            let (temp, delta) = update?;
            let (bb, mir::Rvalue::Use(op, _)) = writes[0] else {
                return None;
            };
            let p = op.place()?;
            if p.local != temp
                || !matches!(p.projection.as_ref(),[mir::ProjectionElem::Field(i,_)] if i.as_usize()==0)
                || !dom.dominates(bb, backedges[0])
            {
                return None;
            }
            delta
        };
        let source = source?;
        if self.body.basic_blocks.iter_enumerated().any(|(bb, data)| inside(bb)
            && data.statements.iter().any(|s| matches!(&s.kind,
                mir::StatementKind::Assign(box (to, _)) if to.local == source || to.local == iterator)))
        {
            return None;
        }
        if !self.body.args_iter().any(|a| a == source)
            || !matches!(self.local_decls[source].ty.kind(),ty::TyKind::Ref(_,t,ty::Mutability::Not) if t.is_slice())
        {
            return None;
        }
        Some(CountShape {
            source,
            counter,
            iterator,
            remaining,
            delta,
            selected_contracts,
            iterator_ty,
        })
    }

    pub(super) fn infer_count_invariants(
        &mut self,
        shape: CountShape<'vir>,
    ) -> EncodeResult<'vir, Vec<vir::ExprBool<'vir>>, E> {
        let inference = match affine::infer(
            &affine::CountLoop {
                counter_delta: shape.delta,
            },
            &config::smt_solver_path(),
        ) {
            Ok(result) => result,
            Err(reason) => {
                tracing::debug!("loop inference declined: {reason}");
                return Ok(Vec::new());
            }
        };
        let vcx = self.vcx;
        let tcx = vcx.tcx();
        let source = self.local_defs[shape.source].impure_snap;
        let counter = self.local_defs[shape.counter].impure_snap;
        let iterator = self.local_defs[shape.iterator].impure_snap;
        let usize_ty = self.ty_use_pure(tcx.types.usize);
        let count = usize_ty
            .expect_primitive()
            .snap_to_prim(counter.downcast_ty())
            .downcast_ty::<vir::Int>();
        let source_ty = self.ty_use_pure(self.local_decls[shape.source].ty);
        let metadata = source_ty
            .expect_immref()
            .metadata_access(source.downcast_ty());
        let length = usize_ty
            .expect_primitive()
            .snap_to_prim(metadata.downcast_ty())
            .downcast_ty::<vir::Int>();
        let reference_ty = ty::Ty::new_imm_ref(tcx, tcx.lifetimes.re_erased, shape.iterator_ty);
        let reference = self.ty_use_pure(reference_ty);
        let unit = self
            .ty_use_pure(tcx.types.unit)
            .expect_structlike()
            .field_snaps_to_snap(Vec::new());
        let argument = reference
            .expect_immref()
            .prim_to_snap(vcx.mk_null(), unit.upcast_ty(), iterator)
            .upcast_ty();
        let function = self
            .deps
            .require_dep::<FunctionCallEnc>(CallTaskDescription::new(
                self.def_id,
                tcx.mk_args(&[shape.iterator_ty.into()]),
                shape.remaining,
            ))?;
        let sequence = function.call_pure(vec![argument]);
        let remaining = vcx.mk_collection_len_expr(sequence);
        let observations = [count, remaining, length];
        let mut result = vec![vcx.mk_eq_expr(source, vcx.mk_old_expr(source))];
        for candidate in &inference.survivors {
            let mut sum = vcx.mk_int::<0>();
            for (coefficient, value) in candidate.coefficients.iter().zip(observations) {
                sum = match coefficient {
                    1 => vcx
                        .mk_bin_op_expr(vir::BinOpKind::Add, sum, value)
                        .downcast_ty(),
                    -1 => vcx
                        .mk_bin_op_expr(vir::BinOpKind::Sub, sum, value)
                        .downcast_ty(),
                    0 => sum,
                    _ => unreachable!(),
                };
            }
            result.push(if candidate.equality {
                vcx.mk_eq_expr(sum, vcx.mk_int::<0>())
            } else {
                vcx.mk_bin_op_expr(vir::BinOpKind::CmpGe, sum, vcx.mk_int::<0>())
                    .downcast_ty()
            });
        }
        let dir = config::log_dir().join("loop-inference");
        if std::fs::create_dir_all(&dir).is_ok() {
            let name = format!(
                "{}-{}",
                self.def_id.krate.as_u32(),
                self.def_id.index.as_u32()
            );
            let _ = std::fs::write(dir.join(format!("{name}.smt2")), &inference.transcript);
            let _ = std::fs::write(
                dir.join(format!("{name}.txt")),
                format!(
                    "function={:?}\nsource={:?}\ncounter={:?}\niterator={:?}\ncontracts={:?}\nremaining={:?}\ndelta={}\ngenerated={}\nqueries={}\nseconds={}\nsurvivors={:?}\n",
                    self.def_id,
                    shape.source,
                    shape.counter,
                    shape.iterator,
                    shape.selected_contracts,
                    shape.remaining,
                    shape.delta,
                    inference.generated,
                    inference.queries,
                    inference.elapsed.as_secs_f64(),
                    inference.survivors
                ),
            );
        }
        Ok(result)
    }
}
