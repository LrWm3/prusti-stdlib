#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use prusti_contracts::*;

// Verified definitions, exported to a separate client crate. No element bounds.
#[pure]
#[ensures(result == Seq::from_slice(value))]
pub fn slice_contents<T>(value: &[T]) -> Seq<T> {
    Seq::from_slice(value)
}

#[pure]
#[ensures(result == prusti_contracts::alloc_spec::vec::vec_contents(value))]
pub fn vec_contents<T>(value: &Vec<T>) -> Seq<T> {
    Seq::from_slice(value.as_slice())
}

#[ensures(prusti_contracts::alloc_spec::vec::vec_contents(dest) == old(prusti_contracts::alloc_spec::vec::vec_contents(&src)))]
#[ensures(prusti_contracts::alloc_spec::vec::vec_contents(&result) == old(prusti_contracts::alloc_spec::vec::vec_contents(&*dest)))]
pub fn replace_vec<T>(dest: &mut Vec<T>, src: Vec<T>) -> Vec<T> {
    core::mem::replace(dest, src)
}
