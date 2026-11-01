use crate::{
    core_spec::slice::{range_index_model, scalar_index_model},
    iter_model::ModeledIterator,
    *,
};
use alloc::vec::Vec;
use core::{
    ops::{Deref, DerefMut, Index, IndexMut},
    slice::SliceIndex,
};

// Vec::into_iter transfers the initialized elements in their original order.
#[extern_spec]
impl<T, A: core::alloc::Allocator> core::iter::IntoIterator for Vec<T, A> {
    #[trusted]
    #[ensures(crate::iter_model::ModeledIterator::remaining(&result) == vec_contents(&self))]
    #[ensures({
        let source = old(vec_contents(&self));
        let tail = result.remaining();
        forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
            sequence_resource_state(&tail, j) == old(sequence_resource_state(&source, j)),
            triggers = [(tail[j],)])
    })]
    #[after_expiry(self[_] => {
        let source = old(vec_contents(&self));
        let tail = after_call(result.remaining());
        forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
            sequence_resource_state(&source, j) == before_expiry(sequence_resource_state(&tail, j)),
            triggers = [(source[j],)])
    })]
    fn into_iter(self) -> alloc::vec::IntoIter<T, A>;
}

// The pinned IntoIter::next moves the first element out and advances the
// logical tail. Neither this move nor its permission restoration changes
// referents owned by those elements, including zero-sized elements.
#[extern_spec]
impl<T, A: core::alloc::Allocator> Iterator for alloc::vec::IntoIter<T, A> {
    #[trusted]
    #[ensures(match &result {
        Some(value) => old(self.remaining()).len() > Int::from(0)
            && *value === *old(self.remaining())[0]
            && self.remaining() == old(self.remaining())[1..],
        None => old(self.remaining()).len() == Int::from(0)
            && self.remaining() == old(self.remaining()),
    })]
    #[ensures({
        let source = old(self.remaining());
        match &result {
            Some(value) => resource_state(value)
                == old(sequence_resource_state(&source, Int::from(0))),
            None => true,
        }
    })]
    #[ensures({
        let source = old(self.remaining());
        let tail = self.remaining();
        forall(move |j: Int| (Int::from(0) <= j && j < tail.len()) ==>
            sequence_resource_state(&tail, j)
                == old(sequence_resource_state(&source, j + Int::from(1))),
            triggers = [(tail[j],)])
    })]
    #[after_expiry(self[_] => {
        let source = old(self.remaining());
        match &result {
            Some(value) => sequence_resource_state(&source, Int::from(0))
                == before_expiry(resource_state(value)),
            None => true,
        }
    })]
    #[after_expiry(self[_] => {
        let source = old(self.remaining());
        let tail = after_call(self.remaining());
        forall(move |j: Int| (Int::from(1) <= j && j < source.len()) ==>
            sequence_resource_state(&source, j)
                == before_expiry(sequence_resource_state(&tail, j - Int::from(1))),
            triggers = [(source[j],)])
    })]
    fn next(&mut self) -> Option<T>;
}

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
    #[ensures(Seq::from_slice(result) == vec_contents(self))]
    fn as_slice(&self) -> &[T];

    #[trusted]
    #[ensures(Seq::from_slice(&*result) == old(vec_contents(&*self)))]
    #[after_expiry(self => vec_contents(self) == before_expiry(Seq::from_slice(&*result)))]
    #[ensures({
        let source = old(vec_contents(&*self));
        let view = Seq::from_slice(&*result);
        forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
            sequence_resource_state(&view, j) == old(sequence_resource_state(&source, j)),
            triggers = [(view[j],)])
    })]
    #[after_expiry(self[_] => {
        let source = old(vec_contents(&*self));
        forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
            sequence_resource_state(&source, j) == before_expiry(sequence_resource_state(&source, j)),
            triggers = [(source[j],)])
    })]
    fn as_mut_slice(&mut self) -> &mut [T];

    #[trusted]
    #[pure]
    #[ensures(result == (self.len() == 0))]
    fn is_empty(&self) -> bool;

    #[trusted]
    #[requires(self.len() < usize::MAX)]
    #[ensures(vec_contents(self) == old(vec_contents(&*self)).append(Seq::single_ref(&value)))]
    #[ensures({
        let source = old(vec_contents(&*self));
        let current = vec_contents(&*self);
        forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
            sequence_resource_state(&current, j) == old(sequence_resource_state(&source, j)),
            triggers = [(current[j],)])
    })]
    #[ensures(sequence_resource_state(&vec_contents(&*self), old(vec_contents(&*self)).len())
        == old(resource_state(&value)))]
    #[after_expiry(self[_] => {
        let source = old(vec_contents(&*self));
        let current = after_call(vec_contents(&*self));
        forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
            sequence_resource_state(&source, j) == before_expiry(sequence_resource_state(&current, j)),
            triggers = [(source[j],)])
    })]
    #[after_expiry(value[_] => {
        let current = after_call(vec_contents(&*self));
        let index = old(vec_contents(&*self)).len();
        resource_state(&value) == before_expiry(sequence_resource_state(&current, index))
    })]
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
impl<T, A: core::alloc::Allocator> DerefMut for Vec<T, A> {
    #[trusted]
    #[ensures(Seq::from_slice(&*result) == old(vec_contents(&*self)))]
    #[after_expiry(self => vec_contents(self) == before_expiry(Seq::from_slice(&*result)))]
    #[ensures({
        let source = old(vec_contents(&*self));
        let view = Seq::from_slice(&*result);
        forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
            sequence_resource_state(&view, j) == old(sequence_resource_state(&source, j)),
            triggers = [(view[j],)])
    })]
    #[after_expiry(self[_] => {
        let source = old(vec_contents(&*self));
        forall(move |j: Int| (Int::from(0) <= j && j < source.len()) ==>
            sequence_resource_state(&source, j) == before_expiry(sequence_resource_state(&source, j)),
            triggers = [(source[j],)])
    })]
    fn deref_mut(&mut self) -> &mut [T];
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
        Some(previous) => Ghost::new_ref(&*result) == Ghost::new_ref(previous)
            && core::intrinsics::ptr_metadata(&*result) === core::intrinsics::ptr_metadata(previous),
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
    #[after_expiry(match range_index_model(&index) {
        Some((start, end)) => start <= end ==>
            (start <= self.len() ==>
                vec_contents(&*self)[..start] == old(vec_contents(&*self))[..start])
            && (end <= self.len() ==>
                vec_contents(&*self)[end..] == old(vec_contents(&*self))[end..]),
        None => true,
    })]
    fn index_mut(&mut self, index: I) -> &mut I::Output;
}
