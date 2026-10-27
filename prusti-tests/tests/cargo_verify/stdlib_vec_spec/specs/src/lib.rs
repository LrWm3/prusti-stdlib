#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use prusti_contracts::*;

#[pure]
#[ensures(result == prusti_contracts::alloc_spec::vec::vec_contents(value))]
pub fn view<T>(value: &Vec<T>) -> Seq<T> {
    Seq::from_slice(value.as_slice())
}

#[requires(value.len() < usize::MAX)]
#[ensures(view(value) == old(view(&*value)).append(Seq::single_ref(&item)))]
pub fn append_one<T>(value: &mut Vec<T>, item: T) {
    value.push(item);
}

#[requires(value.len() < usize::MAX)]
#[ensures(result === item)]
#[ensures(view(value) == old(view(&*value)))]
pub fn round_trip<T>(value: &mut Vec<T>, item: T) -> T {
    value.push(item);
    value.pop().unwrap()
}
