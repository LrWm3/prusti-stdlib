use crate::*;

use core::{
    ops::{Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    slice::SliceIndex,
};

#[extern_spec]
impl<T> [T] {
    #[trusted]
    #[pure]
    #[ensures(result == core::intrinsics::ptr_metadata(self))]
    fn len(&self) -> usize;

    #[trusted]
    #[pure]
    #[ensures(result == (self.len() == 0))]
    fn is_empty(&self) -> bool;

    #[trusted]
    #[pure]
    #[ensures(result === index.get(self))]
    fn get<I: SliceIndex<Self>>(&self, index: I) -> Option<&I::Output>;

    #[trusted]
    #[pure]
    #[requires(mid <= self.len())]
    #[ensures(Seq::from_slice(result.0) == Seq::from_slice(self)[..mid])]
    #[ensures(Seq::from_slice(result.1) == Seq::from_slice(self)[mid..])]
    fn split_at(&self, mid: usize) -> (&[T], &[T]);

    #[trusted]
    #[requires(a < self.len())]
    #[requires(b < self.len())]
    #[ensures(Seq::from_slice(self) == old(Seq::from_slice(&*self))
        .update(a, &*old(Seq::from_slice(&*self))[b])
        .update(b, &*old(Seq::from_slice(&*self))[a]))]
    fn swap(&mut self, a: usize, b: usize);
}

#[extern_spec]
impl<T, I: SliceIndex<[T]>> Index<I> for [T] {
    #[trusted]
    #[pure]
    #[requires(matches!(index.get(self), Some(_)))]
    #[ensures(match index.get(self) {
        Some(value) => result === value,
        None => false,
    })]
    fn index(&self, index: I) -> &Self::Output;
}

#[extern_spec]
pub unsafe trait SliceIndex<T>
where
    T: ?Sized,
{
    #[trusted]
    #[pure]
    fn get(self, slice: &T) -> Option<&Self::Output>;

    #[trusted]
    #[pure]
    fn index(self, slice: &T) -> &Self::Output;
}

#[extern_spec]
impl<T> SliceIndex<[T]> for usize {
    #[trusted]
    #[pure]
    #[ensures(match &result {
        None => self >= slice.len(),
        Some(value) => self < slice.len() && **value === slice[self],
    })]
    fn get(self, slice: &[T]) -> Option<&T>;
}

#[extern_spec]
impl<T> SliceIndex<[T]> for Range<usize> {
    #[trusted]
    #[pure]
    #[ensures(match &result {
        None => self.start > self.end || self.end > slice.len(),
        Some(value) => self.start <= self.end && self.end <= slice.len()
            && Seq::from_slice(value) == Seq::from_slice(slice)[self.start..self.end],
    })]
    fn get(self, slice: &[T]) -> Option<&[T]>;
}

#[extern_spec]
impl<T> SliceIndex<[T]> for RangeFrom<usize> {
    #[trusted]
    #[pure]
    #[ensures(match &result {
        None => self.start > slice.len(),
        Some(value) => self.start <= slice.len()
            && Seq::from_slice(value) == Seq::from_slice(slice)[self.start..],
    })]
    fn get(self, slice: &[T]) -> Option<&[T]>;
}

#[extern_spec]
impl<T> SliceIndex<[T]> for RangeTo<usize> {
    #[trusted]
    #[pure]
    #[ensures(match &result {
        None => self.end > slice.len(),
        Some(value) => self.end <= slice.len()
            && Seq::from_slice(value) == Seq::from_slice(slice)[..self.end],
    })]
    fn get(self, slice: &[T]) -> Option<&[T]>;
}

#[extern_spec]
impl<T> SliceIndex<[T]> for RangeFull {
    #[trusted]
    #[pure]
    #[ensures(match &result {
        None => false,
        Some(value) => Seq::from_slice(value) == Seq::from_slice(slice),
    })]
    fn get(self, slice: &[T]) -> Option<&[T]>;
}

#[extern_spec]
impl<T> SliceIndex<[T]> for RangeToInclusive<usize> {
    #[trusted]
    #[pure]
    #[ensures(match &result {
        None => self.end >= slice.len(),
        Some(value) => self.end < slice.len()
            && Seq::from_slice(value) == Seq::from_slice(slice)[..Int::from(self.end) + Int::from(1)],
    })]
    fn get(self, slice: &[T]) -> Option<&[T]>;
}

#[extern_spec]
impl<T> SliceIndex<[T]> for RangeInclusive<usize> {
    #[trusted]
    #[pure]
    // The constructor snapshot distinguishes a fresh range from an exhausted
    // iterator even when their endpoints coincide. Iterator state is P2-C.
    #[ensures(self === RangeInclusive::new(*self.start(), *self.end()) ==> match &result {
        None => *self.end() >= slice.len()
            || Int::from(*self.start()) > Int::from(*self.end()) + Int::from(1),
        Some(value) => *self.end() < slice.len()
            && Int::from(*self.start()) <= Int::from(*self.end()) + Int::from(1)
            && Seq::from_slice(value) == Seq::from_slice(slice)[Int::from(*self.start())..Int::from(*self.end()) + Int::from(1)],
    })]
    fn get(self, slice: &[T]) -> Option<&[T]>;
}
