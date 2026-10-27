use crate::*;
use core::ops::RangeInclusive;

#[extern_spec]
impl<Idx> RangeInclusive<Idx> {
    #[trusted]
    #[pure]
    #[ensures(*result.start() === start)]
    #[ensures(*result.end() === end)]
    fn new(start: Idx, end: Idx) -> Self;

    #[trusted]
    #[pure]
    fn start(&self) -> &Idx;

    #[trusted]
    #[pure]
    fn end(&self) -> &Idx;
}
