// Contracts for the native iterator's logical borrowed tail.
// Reviewed against the pinned core::slice implementation: next removes one
// leading element, leaves the remaining elements unchanged, and is exhausted
// exactly when the tail is empty. Expiry recombines the final yielded value
// with the final tail. No physical-address distinctness is assumed.
use crate::*;

#[extern_spec]
impl<T> [T] {
    #[trusted]
    #[ensures(mut_slice_len(&*mut_iter_tail(&result)) == old(self.len()))]
    #[ensures(mut_slice_contents(&*mut_iter_tail(&result)) == old(Seq::from_slice(&*self)))]
    #[after_expiry(self => Seq::from_slice(&*self) == before_expiry(mut_slice_contents(&*mut_iter_tail(&result))))]
    #[ensures(mut_iter_refs(&result).len() == Int::from(old(self.len())))]
    #[ensures(forall(|i: usize| i < old(self.len()) ==>
        mut_ref_shared_same_address(&*mut_iter_refs(&result)[i], old(&self[i]))))]
    fn iter_mut(&mut self) -> core::slice::IterMut<'_, T>;
}

#[extern_spec]
impl<'a, T> Iterator for core::slice::IterMut<'a, T> {
    #[trusted]
    #[ensures(match &result {
        Some(value) => old(mut_slice_len(&*mut_iter_tail(&*self))) > 0
            && mut_ref_same_address(value, &*old(mut_iter_refs(&*self))[0])
            && mut_slice_len(&*mut_iter_tail(&*self)) + 1 == old(mut_slice_len(&*mut_iter_tail(&*self)))
            && Ghost::new_ref(&**value) == old(mut_slice_contents(&*mut_iter_tail(&*self)))[0],
        None => old(mut_slice_len(&*mut_iter_tail(&*self))) == 0
            && mut_slice_len(&*mut_iter_tail(&*self)) == 0,
    })]
    #[ensures(match &result {
        Some(_) => mut_slice_contents(&*mut_iter_tail(&*self)) == old(mut_slice_contents(&*mut_iter_tail(&*self)))[1..],
        None => mut_slice_contents(&*mut_iter_tail(&*self)) == old(mut_slice_contents(&*mut_iter_tail(&*self))),
    })]
    #[after_expiry(*self => match (old(mut_iter_tail(&*self)), after_call(mut_iter_tail(&*self)), &result) {
        (slice, tail, Some(value)) => mut_slice_contents(&*slice)
            == Seq::single(&*before_expiry(Ghost::new_ref(&**value)))
                .append(before_expiry(mut_slice_contents(&*tail))),
        _ => true,
    })]
    #[after_expiry(*self => match (old(mut_iter_tail(&*self)), after_call(mut_iter_tail(&*self)), &result) {
        (slice, tail, None) => mut_slice_contents(&*slice)
            == before_expiry(mut_slice_contents(&*tail)),
        _ => true,
    })]
    #[ensures(match &result {
        Some(_) => mut_iter_refs(&*self) == old(mut_iter_refs(&*self))[1..],
        None => mut_iter_refs(&*self) == old(mut_iter_refs(&*self)),
    })]
    fn next(&mut self) -> Option<&'a mut T>;
}

// Mutable-slice iteration constructs the same borrowed tail as iter_mut.
#[extern_spec]
impl<'a, T> core::iter::IntoIterator for &'a mut [T] {
    #[trusted]
    #[ensures(mut_slice_len(&*mut_iter_tail(&result)) == old(self.len()))]
    #[ensures(mut_slice_contents(&*mut_iter_tail(&result)) == old(Seq::from_slice(&*self)))]
    #[after_expiry(self => Seq::from_slice(&*self) == before_expiry(mut_slice_contents(&*mut_iter_tail(&result))))]
    #[ensures(mut_iter_refs(&result).len() == Int::from(old(self.len())))]
    #[ensures(forall(|i: usize| i < old(self.len()) ==>
        mut_ref_shared_same_address(&*mut_iter_refs(&result)[i], old(&self[i]))))]
    fn into_iter(self) -> core::slice::IterMut<'a, T>;
}

#[extern_spec]
impl<'a, T> core::iter::ExactSizeIterator for core::slice::IterMut<'a, T> {
    #[trusted]
    #[pure]
    #[ensures(result == mut_slice_len(&*mut_iter_tail(self)))]
    fn len(&self) -> usize;
}
