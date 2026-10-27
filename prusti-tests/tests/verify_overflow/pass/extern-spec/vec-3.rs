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
