//! Cancel explicit snapshot constructor/projection pairs, including in wands.
//! This does not substitute bindings, move heap reads between states, or
//! change the order of permission conjuncts. Apply it uniformly to declarations,
//! packages, and applications so their wand shapes remain identical.
use super::{walk::Simplifier, SimplifyCtx};
use crate::{gendata::*, refs::*, Foldable, Folder, VirCtxt};
use prusti_rustc_interface::data_structures::fx::FxHashMap;

pub(super) fn normalize<'vir, T: Foldable<'vir, (), !> + Copy>(
    vcx: &'vir VirCtxt<'_>,
    ctx: &SimplifyCtx<'vir>,
    item: T,
) -> T {
    let mut folder = Projections {
        vcx,
        ctx,
        reveals: Simplifier::new(vcx, ctx),
        memo: FxHashMap::default(),
    };
    item.fold_with(&mut folder).unwrap_or(item)
}

struct Projections<'a, 'vir, 'tcx> {
    vcx: &'vir VirCtxt<'tcx>,
    ctx: &'a SimplifyCtx<'vir>,
    reveals: Simplifier<'a, 'vir, 'tcx>,
    memo: FxHashMap<*const ExprGenData<'vir, (), !, crate::Dyn>, Option<ExprDyn<'vir>>>,
}

impl<'vir> Folder<'vir, (), !> for Projections<'_, 'vir, '_> {
    fn fold_expr(&mut self, e: ExprDyn<'vir>) -> Option<ExprDyn<'vir>> {
        let key = std::ptr::from_ref(e);
        if let Some(result) = self.memo.get(&key) {
            return *result;
        }
        let children = e.super_fold_with(self);
        let current = children.unwrap_or(e);
        let projected = (|| {
            let ExprKindGenData::AdtDestructor(receiver, field) = current.kind else {
                return None;
            };
            let ExprKindGenData::FuncApp(app) = receiver.kind else {
                return None;
            };
            let &(constructor, index) = self.ctx.destructors.get(field.name)?;
            let arg = app.args.get(index)?;
            if app.target != constructor || receiver.ty() != field.input || arg.ty() != field.ty {
                return None;
            }
            // Even an unused checked reveal must stay at its original
            // evaluation point: it supplies a proof fact there.
            if self.reveals.has_reveal(receiver) {
                return None;
            }
            Some(self.vcx.alloc(ExprGenData::new_inner(
                arg.kind,
                e.debug_info,
                e.span,
                e.ty(),
            )))
        })();
        let result = projected.or(children);
        self.memo.insert(key, result);
        result
    }
}
