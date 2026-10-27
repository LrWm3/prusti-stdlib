use crate::{core_spec::slice::scalar_index_model, *};
use alloc::vec::Vec;
use core::{
    ops::{Deref, Index, IndexMut},
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
    #[ensures(Seq::from_slice(&*result) == old(vec_contents(&*self)))]
    #[after_expiry(vec_contents(self) == before_expiry(Seq::from_slice(&*result)))]
    fn as_mut_slice(&mut self) -> &mut [T];

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

    #[trusted]
    #[ensures(vec_contents(self) == Seq::new())]
    fn clear(&mut self);

    #[trusted]
    #[ensures(vec_contents(self) == if len < old(self.len()) {
        old(vec_contents(&*self))[..len]
    } else {
        old(vec_contents(&*self))
    })]
    fn truncate(&mut self, len: usize);

    #[trusted]
    #[requires(Int::from(self.len()) + Int::from(other.len()) <= Int::from(usize::MAX))]
    #[ensures(vec_contents(self) == old(vec_contents(&*self)).append(old(vec_contents(&*other))))]
    #[ensures(vec_contents(other) == Seq::new())]
    fn append(&mut self, other: &mut Self);

    #[trusted]
    #[requires(index <= self.len())]
    #[requires(self.len() < usize::MAX)]
    #[ensures(vec_contents(self) == old(vec_contents(&*self))[..index]
        .append(Seq::single_ref(&element)).append(old(vec_contents(&*self))[index..]))]
    fn insert(&mut self, index: usize, element: T);

    #[trusted]
    #[requires(index < self.len())]
    #[ensures(result === *old(vec_contents(&*self))[index])]
    #[ensures(vec_contents(self) == old(vec_contents(&*self))[..index]
        .append(old(vec_contents(&*self))[Int::from(index) + Int::from(1)..]))]
    fn remove(&mut self, index: usize) -> T;

    #[trusted]
    #[requires(index < self.len())]
    #[ensures(result === *old(vec_contents(&*self))[index])]
    #[ensures(vec_contents(self) == if Int::from(index) == Int::from(old(self.len())) - Int::from(1) {
        old(vec_contents(&*self))[..index]
    } else {
        old(vec_contents(&*self))[..Int::from(old(self.len())) - Int::from(1)]
            .update(index, &*old(vec_contents(&*self))[Int::from(old(self.len())) - Int::from(1)])
    })]
    fn swap_remove(&mut self, index: usize) -> T;
}

#[extern_spec]
impl<T, A: core::alloc::Allocator> Deref for Vec<T, A> {
    #[trusted]
    #[pure]
    #[ensures(Seq::from_slice(result) == vec_contents(self))]
    fn deref(&self) -> &Self::Target;
}

#[extern_spec]
impl<T, I: SliceIndex<[T]>, A: core::alloc::Allocator> Index<I> for Vec<T, A> {
    #[trusted]
    #[pure]
    #[requires(matches!(index.get(self.as_slice()), Some(_)))]
    #[ensures(match index.get(self.as_slice()) {
        Some(value) => result === value,
        None => false,
    })]
    fn index(&self, index: I) -> &Self::Output;
}

#[extern_spec]
impl<T, I: SliceIndex<[T]>, A: core::alloc::Allocator> IndexMut<I> for Vec<T, A> {
    #[trusted]
    #[requires(matches!(index.get(self.as_slice()), Some(_)))]
    #[ensures(match old(index.get(self.as_slice())) {
        Some(previous) => Ghost::new_ref(&*result) == Ghost::new_ref(previous),
        None => false,
    })]
    #[after_expiry(self.len() == old(self.len()) && match index.get(self.as_slice()) {
        Some(current) => Ghost::new_ref(current) == before_expiry(Ghost::new_ref(&*result)),
        None => false,
    })]
    #[after_expiry(match scalar_index_model(&index) {
        Some(position) => position < self.len() ==>
            vec_contents(&*self) == old(vec_contents(&*self))
                .update(position, &*vec_contents(&*self)[position]),
        None => true,
    })]
    fn index_mut(&mut self, index: I) -> &mut I::Output;
}
