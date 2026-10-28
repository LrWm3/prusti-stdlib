// Test-only contracts for the native iterator's logical borrowed tail.
// Reviewed against the pinned core::slice implementation: next removes one
// leading element, leaves the remaining elements unchanged, and is exhausted
// exactly when the tail is empty. Expiry recombines the final yielded value
// with the final tail. No physical-address distinctness is assumed.
use prusti_contracts::*;

#[extern_spec]
impl<T> [T] {
    #[trusted]
    #[ensures(mut_slice_len(&*mut_iter_tail(&result)) == old(self.len()))]
    #[ensures(mut_slice_contents(&*mut_iter_tail(&result)) == old(Seq::from_slice(&*self)))]
    #[after_expiry(Seq::from_slice(&*self) == before_expiry(mut_slice_contents(&*mut_iter_tail(&result))))]
    fn iter_mut(&mut self) -> core::slice::IterMut<'_, T>;
}

#[extern_spec]
impl<'a, T> Iterator for core::slice::IterMut<'a, T> {
    #[trusted]
    #[ensures(match &result {
        Some(value) => old(mut_slice_len(&*mut_iter_tail(&*self))) > 0
            && mut_slice_len(&*mut_iter_tail(&*self)) + 1 == old(mut_slice_len(&*mut_iter_tail(&*self)))
            && Ghost::new_ref(&**value) == old(mut_slice_contents(&*mut_iter_tail(&*self)))[0],
        None => old(mut_slice_len(&*mut_iter_tail(&*self))) == 0
            && mut_slice_len(&*mut_iter_tail(&*self)) == 0,
    })]
    #[ensures(match &result {
        Some(_) => mut_slice_contents(&*mut_iter_tail(&*self)) == old(mut_slice_contents(&*mut_iter_tail(&*self)))[1..],
        None => mut_slice_contents(&*mut_iter_tail(&*self)) == old(mut_slice_contents(&*mut_iter_tail(&*self))),
    })]
    #[after_expiry(match (old(mut_iter_tail(&*self)), after_call(mut_iter_tail(&*self)), &result) {
        (slice, tail, Some(value)) => mut_slice_contents(&*slice)
            == Seq::single(&*before_expiry(Ghost::new_ref(&**value)))
                .append(before_expiry(mut_slice_contents(&*tail))),
        _ => true,
    })]
    #[after_expiry(match (old(mut_iter_tail(&*self)), after_call(mut_iter_tail(&*self)), &result) {
        (slice, tail, None) => mut_slice_contents(&*slice)
            == before_expiry(mut_slice_contents(&*tail)),
        _ => true,
    })]
    fn next(&mut self) -> Option<&'a mut T>;
}
