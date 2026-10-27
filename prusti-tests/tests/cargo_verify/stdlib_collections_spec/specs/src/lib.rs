#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use prusti_contracts::*;

#[pure]
pub fn view<T>(values: &Vec<T>) -> Seq<T> {
    Seq::from_slice(values.as_slice())
}

#[requires(Int::from(dest.len()) + Int::from(src.len()) <= Int::from(usize::MAX))]
#[ensures(view(dest) == old(view(&*dest)).append(old(view(&*src))))]
#[ensures(view(src) == Seq::new())]
pub fn concatenate<T>(dest: &mut Vec<T>, src: &mut Vec<T>) {
    dest.append(src);
}

#[requires(index <= values.len())]
#[requires(values.len() < usize::MAX)]
#[ensures(result === item)]
#[ensures(view(values) == old(view(&*values)))]
pub fn insert_remove<T>(values: &mut Vec<T>, index: usize, item: T) -> T {
    values.insert(index, item);
    values.remove(index)
}

#[requires(mid <= values.len())]
#[ensures(Seq::from_slice(result.0).append(Seq::from_slice(result.1)) == Seq::from_slice(values))]
#[ensures(result.0.len() == mid)]
pub fn split<T>(values: &[T], mid: usize) -> (&[T], &[T]) {
    values.split_at(mid)
}

// No Copy, Clone, PartialEq, or Eq: clients compare value snapshots.
pub struct Owned {
    pub value: usize,
}

#[ensures(result.len() == n)]
#[ensures(forall(|i: usize| i < n ==> *view(&result)[i] === Owned { value: i }))]
pub fn make(n: usize) -> Vec<Owned> {
    let mut values = Vec::new();
    let mut i = 0;
    loop {
        body_invariant!(n == old(n));
        body_invariant!(i <= n);
        body_invariant!(values.len() == i);
        body_invariant!(forall(|j: usize| j < i ==> *view(&values)[j] === Owned { value: j }));
        if i == n {
            break;
        }
        values.push(Owned { value: i });
        i += 1;
    }
    values
}
