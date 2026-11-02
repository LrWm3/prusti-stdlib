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
    ])]
    fn index_mut(&mut self, index: I) -> &mut <[T] as Index<I>>::Output;
}
