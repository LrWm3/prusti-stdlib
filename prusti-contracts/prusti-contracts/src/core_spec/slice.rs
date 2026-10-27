use crate::*;

use core::slice::SliceIndex;

#[extern_spec]
impl<T> [T] {
    #[trusted]
    #[pure]
    #[ensures(result == core::intrinsics::ptr_metadata(self))]
    fn len(&self) -> usize;
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
