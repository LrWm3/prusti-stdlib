use crate::*;
use alloc::vec::Vec;
use core::{
    ops::Index,
    slice::SliceIndex,
};

// These are normal-return contracts for owned elements with stable value
// snapshots. Allocation/capacity failures and arbitrary Drop behavior remain
// trusted execution assumptions; the length precondition prevents usize wrap,
// but does not promise that a buffer allocation can succeed.

/// Observe the element snapshots of a Vec without consuming its elements.
/// This specification helper is separate from the contracts of real methods.
#[pure]
pub fn vec_contents<T, A: core::alloc::Allocator>(value: &Vec<T, A>) -> Seq<T> {
    Seq::from_slice(value.as_slice())
}

#[extern_spec]
impl<T> Vec<T> {
    #[trusted]
    #[ensures(vec_contents(&result) == Seq::new())]
    fn new() -> Self;
}

#[extern_spec]
impl<T, A: core::alloc::Allocator> Vec<T, A> {
    #[trusted]
    #[pure]
    #[ensures(result == self.as_slice().len())]
    fn len(&self) -> usize;

    #[trusted]
    #[pure]
    fn as_slice(&self) -> &[T];

    #[trusted]
    #[pure]
    #[ensures(result == (self.len() == 0))]
    fn is_empty(&self) -> bool;

    #[trusted]
    #[requires(self.len() < usize::MAX)]
    #[ensures(vec_contents(self) == old(vec_contents(&*self)).append(Seq::single_ref(&value)))]
    fn push(&mut self, value: T);

    #[trusted]
    #[ensures(match &result {
        None => old(vec_contents(&*self)).len() == Int::from(0),
        Some(value) => old(vec_contents(&*self)).len() > Int::from(0)
            && *value === *old(vec_contents(&*self))[old(vec_contents(&*self)).len() - Int::from(1)],
    })]
    #[ensures(vec_contents(self) == if old(self.len()) == 0 {
        old(vec_contents(&*self))
    } else {
        old(vec_contents(&*self))[..Int::from(old(self.len())) - Int::from(1)]
    })]
    fn pop(&mut self) -> Option<T>;
}

#[extern_spec]
impl<T, I: SliceIndex<[T]>, A: core::alloc::Allocator> Index<I> for Vec<T, A> {
    #[trusted]
    #[pure]
    #[requires(matches!(index.get(self.as_slice()), Some(_)))]
    #[ensures(match index.get(self.as_slice()) {
        Some(value) => *result === *value,
        None => false,
    })]
    fn index(&self, index: I) -> &Self::Output;
}
