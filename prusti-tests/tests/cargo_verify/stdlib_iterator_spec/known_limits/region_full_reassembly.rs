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
    #[after_expiry(match (old(&self.tail), after_call(&self.tail), &result) {
        (Some(slice), Some(tail), Some(value)) => mut_slice_contents(slice)
            == Seq::single(&*before_expiry(Ghost::new_ref(&**value)))
                .append(before_expiry(mut_slice_contents(tail))),
        (Some(slice), Some(tail), None) => mut_slice_contents(slice) == before_expiry(mut_slice_contents(tail)),
        (Some(slice), None, Some(value)) => mut_slice_contents(slice) == Seq::single(&*before_expiry(Ghost::new_ref(&**value))),
        (Some(slice), None, None) => mut_slice_contents(slice) == Seq::new(),
        (None, _, _) => true,
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
#[requires(values.len() == 2)]
fn visit(values: &mut [u32]) {
    let mut it = Iter { tail: Some(values) };
    prusti_assert!(remaining(&it) == 2);
    let first = it.next();
    prusti_assert!(remaining(&it) == 1);
    let second = it.next();
    prusti_assert!(remaining(&it) == 0);
    match (first, second) {
        (Some(first), Some(second)) => { *first = 7; *second = 8; }
        _ => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values.len() == 2);
    prusti_assert!(values[0] == 7);
    prusti_assert!(values[1] == 8);
}
fn main() { let mut values = [1, 2]; visit(&mut values); }
