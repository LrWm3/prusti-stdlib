#![feature(allocator_api)]

use prusti_contracts::*;
use std::vec::Vec;
use std::option::Option;

#[extern_spec]
impl<T> Option<T> {
    #[trusted]
    #[pure]
    #[ensures(matches!(*self, Some(_)) == result)]
    pub fn is_some(&self) -> bool;

    #[trusted]
    #[pure]
    #[ensures(self.is_some() ==> !result)]
    #[ensures(!self.is_some() ==> result)]
    pub fn is_none(&self) -> bool;

}

#[extern_spec]
impl<T, A: std::alloc::Allocator> Vec<T, A> {
    #[trusted]
    #[ensures(self.len() == 0)]
    fn clear(&mut self);
}

#[extern_spec]
impl<T, A: std::alloc::Allocator> Vec<T, A> {
    #[trusted]
    #[ensures(self.len() == old(self.len()) + other.len())]
    pub fn append(&mut self, other: &mut Vec<T, A>);
}

fn main() {
    let mut v = Vec::new();
    v.push(2);
    v.push(3);
    v.push(5);
    assert!(v.len() == 3);
    v.pop();
    assert!(v.len() == 2);
    v.pop();
    v.pop();
    assert!(v.pop().is_none());
}
