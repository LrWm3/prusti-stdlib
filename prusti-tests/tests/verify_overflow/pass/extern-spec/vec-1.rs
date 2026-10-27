#![feature(allocator_api)]

use prusti_contracts::*;

fn main() {
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    assert!(v.len() == 3);
    v.clear();
    assert!(v.len() == 0);
}
