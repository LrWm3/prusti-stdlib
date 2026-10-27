#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use prusti_contracts::*;

#[pure]
pub fn view<Element>(values: &Vec<Element>) -> Seq<Element> {
    Seq::from_slice(values.as_slice())
}

#[requires(first < values.len())]
#[requires(second < values.len())]
#[ensures(view(values) == old(view(&*values))
    .update(first, &*old(view(&*values))[second])
    .update(second, &*old(view(&*values))[first]))]
pub fn swap_view<Element>(values: &mut Vec<Element>, first: usize, second: usize) {
    values.as_mut_slice().swap(first, second);
}
