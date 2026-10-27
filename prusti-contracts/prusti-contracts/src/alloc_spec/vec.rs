use crate::*;
use alloc::vec::Vec;

/// Observe the element snapshots of a Vec without consuming its elements.
/// This specification helper is separate from the contracts of real methods.
#[pure]
pub fn vec_contents<T, A: core::alloc::Allocator>(value: &Vec<T, A>) -> Seq<T> {
    Seq::from_slice(value.as_slice())
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
}
