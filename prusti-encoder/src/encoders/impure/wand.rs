use pcg::{borrow_pcg::state::BorrowsState, coupling::PcgCoupledEdgeKind, pcg::PcgNode};
use prusti_interface::PrustiError;
use task_encoder::TaskEncoder;

use crate::encoders::{EncodeResult, ImpureEncVisitor, mir_impure::Footprint};

use super::r#loop::WandOldOuter;

/// The inputs of a coupled edge with the resources each is given back as.
type Resources<'a> = Vec<(PcgNode<'a>, Vec<vir::ExprBool<'a>>)>;

impl<'vir, 'enc, E: TaskEncoder> ImpureEncVisitor<'vir, 'enc, E> {
    pub(crate) fn pcs_handle_wand(
        &mut self,
        borrows_state: &BorrowsState<'_, 'vir>,
        package: bool,
        edge: &PcgCoupledEdgeKind<'vir>,
        label: Option<&'vir str>,
        edge_to_loop: bool,
        footprint: Option<&mut Footprint<'_, 'vir>>,
    ) -> EncodeResult<'vir, (), E> {
        // TODO: there is something in the pcs which emits spurious? opaque edge creation, skip these
        if package && !edge_to_loop {
            return Ok(());
        }
        let mut old_outer = WandOldOuter::Label(label);
        let (wand, rhs) = self.coupled_wand(edge, &mut old_outer)?;
        let wand =
            self.with_restoration_wand(wand, edge, &mut old_outer, !package, footprint.is_some())?;
        // P --* true needs no stored obligation. Applying it only consumes P;
        // packaging it establishes no resource or functional fact.
        if matches!(
            wand.rhs.kind,
            vir::ExprKindData::Const(vir::ConstData::Bool(true))
        ) {
            if !package {
                self.stmt(self.vcx.mk_exhale_stmt(wand.lhs));
            }
            return Ok(());
        }
        if !package {
            if let Some(footprint) = footprint {
                footprint.parts.push(self.vcx.mk_wand_expr(wand));
                self.stmt(self.vcx.mk_assert_stmt(wand.lhs));
            }
            let span = edge
                .loop_head()
                .and_then(|head| self.loop_restoration_frames.get(&head))
                .map(|frame| frame.span);
            let stmt = if let Some(span) = span {
                self.vcx.with_span(span, |vcx| {
                    vcx.handle_error("apply.failed:wand.not.found", move |_| {
                        Some(vec![PrustiError::verification(
                            "loop restoration wand might not be available",
                            span.into(),
                        )])
                    });
                    vcx.mk_apply_stmt(wand)
                })
            } else {
                self.vcx.mk_apply_stmt(wand)
            };
            self.stmt(stmt);
            return Ok(());
        }
        let WandOldOuter::Label(label) = &mut old_outer else {
            unreachable!()
        };
        let label = *label.get_or_insert_with(|| self.new_label("outer_package"));
        let proof_block = self.pcs_package_script(borrows_state, rhs, label, &[])?;
        let proof_block = self.vcx.alloc_slice(&proof_block);
        let span = edge
            .loop_head()
            .and_then(|head| self.loop_restoration_frames.get(&head))
            .map(|frame| frame.span);
        let stmt = if let Some(span) = span {
            self.vcx.with_span(span, |vcx| {
                vcx.handle_error("package.failed:assertion.false", move |_| {
                    Some(vec![PrustiError::verification(
                        "loop restoration might not hold",
                        span.into(),
                    )])
                });
                vcx.handle_error("package.failed:insufficient.permission", move |_| {
                    Some(vec![PrustiError::verification(
                        "loop restoration requires additional permission",
                        span.into(),
                    )])
                });
                vcx.mk_package_stmt(wand, proof_block)
            })
        } else {
            self.vcx.mk_package_stmt(wand, proof_block)
        };
        self.stmt(stmt);
        Ok(())
    }

    /// The wand of a coupled edge, and the resources each input is given
    /// back as on its right-hand side.
    pub(super) fn coupled_wand(
        &mut self,
        edge: &PcgCoupledEdgeKind<'vir>,
        old_outer: &mut WandOldOuter<'vir>,
    ) -> EncodeResult<'vir, (vir::Wand<'vir>, Resources<'vir>), E> {
        let inputs = edge.inputs(self.pcg_ctxt());
        let outputs = edge.outputs();
        let mut rhs = Vec::new();
        for i in inputs {
            let mut resources = Vec::new();
            self.encode_pcg_node(&*i, &mut resources, old_outer)?;
            rhs.push((*i, resources));
        }
        let wand_rhs = rhs
            .iter()
            .flat_map(|(_, resources)| resources.iter().copied())
            .collect::<Vec<_>>();
        let mut wand_lhs = Vec::new();
        for i in outputs {
            let i = i.expect_lifetime_projection();
            let exprs = self.encode_lifetime_projection(i, old_outer)?;
            wand_lhs.extend(exprs);
        }
        let wand = self.vcx.mk_wand(
            self.vcx.mk_conj(self.vcx.alloc_slice(&wand_lhs)),
            self.vcx.mk_conj(self.vcx.alloc_slice(&wand_rhs)),
        );
        Ok((wand, rhs))
    }
}
