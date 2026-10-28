use prusti_contracts::*;
struct Iter<'a> { tail: Option<&'a mut [u32]> }
#[pure]
fn remaining(it: &Iter) -> usize {
    match &it.tail { Some(slice) => mut_slice_len(slice), None => 0 }
}
impl<'a> Iter<'a> {
    #[ensures(match &result {
        Some(_) => old(remaining(&*self)) > 0 && remaining(&*self) + 1 == old(remaining(&*self)),
        None => old(remaining(&*self)) == 0 && remaining(&*self) == 0,
    })]
    #[ensures(match (&result, &self.tail) { (Some(_), Some(_)) | (None, _) => true, _ => false })]
    #[ensures(match (old(&self.tail), &self.tail) { (Some(_), Some(_)) | (None, None) => true, _ => false })]
    #[after_expiry(match (old(&self.tail), after_call(&self.tail), &result) {
        (Some(slice), Some(tail), Some(value)) => mut_slice_contents(slice)
            == Seq::single(&*before_expiry(Ghost::new_ref(&**value)))
                .append(before_expiry(mut_slice_contents(tail))),
        _ => true,
    })]
    #[after_expiry(match (old(&self.tail), after_call(&self.tail), &result) {
        (Some(slice), Some(tail), None) => mut_slice_contents(slice) == before_expiry(mut_slice_contents(tail)),
        _ => true,
    })]
    fn next(&mut self) -> Option<&'a mut u32> {
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
#[ensures(forall(|i: usize| i < values.len() ==> values[i] == 7))]
fn visit(values: &mut [u32]) {
    let len = values.len();
    let mut it = Iter { tail: Some(values) };
    loop {
        body_invariant!(remaining(&it) <= len);
        match it.next() { Some(value) => *value = 7, None => break }
    }
    drop(it);
}
fn main() { let mut values = [1, 2]; visit(&mut values); }
