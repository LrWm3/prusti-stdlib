use prusti_contracts::*;
pub struct Iter<'a, T> { pub tail: Option<&'a mut [T]> }
#[pure]
pub fn remaining<T>(it: &Iter<T>) -> usize {
    match &it.tail { Some(slice) => mut_slice_len(slice), None => 0 }
}
impl<'a, T> Iter<'a, T> {
    #[requires(remaining(&*self) > 0)]
    #[ensures(match &result { Some(_) => remaining(&*self) + 1 == old(remaining(&*self)), None => false })]
    #[after_expiry(match (old(&self.tail), after_call(&self.tail), &result) {
        (Some(slice), Some(tail), Some(value)) => mut_slice_contents(slice)
            == Seq::single(&*before_expiry(Ghost::new_ref(&**value)))
                .append(before_expiry(mut_slice_contents(tail))),
        _ => false,
    })]
    pub fn next(&mut self) -> Option<&'a mut T> {
        let slice = match core::mem::replace(&mut self.tail, None) {
            Some(slice) => slice,
            None => return None,
        };
        if slice.is_empty() { self.tail = Some(slice); return None; }
        let (first, rest) = slice.split_at_mut(1);
        self.tail = Some(rest);
        Some(&mut first[0])
    }
}
