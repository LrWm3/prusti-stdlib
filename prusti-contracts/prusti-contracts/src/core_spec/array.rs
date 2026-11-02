use super::slice::{range_index_model, scalar_index_model};
use crate::*;
use core::{
    ops::{Index, IndexMut},
    slice::SliceIndex,
};

// The target also permits custom slice IndexMut implementations. Refine only
// standard SliceIndex calls, matching the target signature and bounds exactly.
#[extern_spec]
impl<T, I, const N: usize> IndexMut<I> for [T; N]
where
    [T]: IndexMut<I>,
{
    #[trusted]
    #[refine_spec(where I: SliceIndex<[T]>, ([T]): Index<I, Output = <I as SliceIndex<[T]>>::Output>, [
        requires(matches!(index.get(&*self), Some(_))),
        ensures(match old(index.get(&*self)) {
        Some(previous) => Ghost::new_ref(&*result) == Ghost::new_ref(previous)
            && core::intrinsics::ptr_metadata(&*result) === core::intrinsics::ptr_metadata(previous),
        None => false,
    }),
        after_expiry(self.len() == old(self.len()) && match index.get(&*self) {
        Some(current) => Ghost::new_ref(current) == before_expiry(Ghost::new_ref(&*result)),
        None => false,
    }),
        after_expiry(match scalar_index_model(&index) {
        Some(position) => position < self.len() ==>
            Seq::from_slice(&*self) == old(Seq::from_slice(&*self))
                .update(position, &*Seq::from_slice(&*self)[position]),
        None => true,
    }),
        after_expiry(match range_index_model(&index) {
        Some((start, end)) => start <= end ==>
            (start <= self.len() ==>
                Seq::from_slice(&*self)[..start] == old(Seq::from_slice(&*self))[..start])
            && (end <= self.len() ==>
                Seq::from_slice(&*self)[end..] == old(Seq::from_slice(&*self))[end..]),
        None => true,
    }),
    ])]
    fn index_mut(&mut self, index: I) -> &mut <[T] as Index<I>>::Output;
}
