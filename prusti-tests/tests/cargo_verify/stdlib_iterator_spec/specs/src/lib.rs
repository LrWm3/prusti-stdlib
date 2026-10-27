#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use prusti_contracts::{iter_model::ModeledIterator, *};

#[pure]
#[ensures(result == prusti_contracts::alloc_spec::vec::vec_contents(value))]
pub fn view<T>(value: &Vec<T>) -> Seq<T> {
    Seq::from_slice(value.as_slice())
}

#[ensures(match &result {
    Some(value) => old(iterator.remaining()).len() > Int::from(0)
        && *value === *old(iterator.remaining())[0]
        && iterator.remaining() == old(iterator.remaining())[1..],
    None => old(iterator.remaining()).len() == Int::from(0)
        && iterator.remaining() == old(iterator.remaining()),
})]
pub fn advance<I: ModeledIterator>(iterator: &mut I) -> Option<I::Item> {
    iterator.next()
}

// The explicit length keeps the pre-move value available as a loop invariant.
#[requires(expected == values.len())]
#[ensures(result == expected)]
pub fn count_owned<T>(values: Vec<T>, expected: usize) -> usize {
    let before = ghost! { view(&values) };
    let mut iterator = values.into_iter();
    let mut count = 0usize;
    loop {
        body_invariant!(expected == old(expected));
        body_invariant!(count <= expected);
        body_invariant!((*before).len() == Int::from(expected));
        body_invariant!(iterator.remaining() == (*before)[count..]);
        match iterator.next() {
            Some(_) => count += 1,
            None => {
                prusti_assert!(count == expected);
                break;
            }
        }
    }
    count
}
