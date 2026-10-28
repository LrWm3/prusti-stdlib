use prusti_contracts::*;
pub struct Iter<'a> { pub tail: Option<&'a mut [u32]> }
#[pure]
pub fn remaining(it: &Iter) -> usize {
    match &it.tail { Some(slice) => mut_slice_len(slice), None => 0 }
}
impl<'a> Iter<'a> {
    #[ensures(match &result {
        Some(_) => old(remaining(&*self)) > 0 && remaining(&*self) + 1 == old(remaining(&*self)),
        None => old(remaining(&*self)) == 0 && remaining(&*self) == 0,
    })]
    #[after_expiry(match old(&self.tail) {
        Some(slice) => match &result {
            Some(value) => mut_slice_contents(slice)[0] == before_expiry(Ghost::new_ref(&**value)),
            None => true,
        },
        None => true,
    })]
    pub fn next(&mut self) -> Option<&'a mut u32> {
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
