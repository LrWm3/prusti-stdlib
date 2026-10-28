use prusti_contracts::*;
#[path = "../support/mut_region_full.rs"]
mod model;
use model::{Iter, remaining};

fn visit(values: &mut [u32]) {
    let len = values.len();
    let mut it = Iter { tail: Some(values) };
    loop {
        body_invariant!(remaining(&it) <= len);
        match it.next() {
            Some(value) => *value = 7,
            None => break,
        }
    }
    drop(it);
    // Reusing the owner requires the loop to return the original borrow.
    if !values.is_empty() {
        values[0] = 11;
        prusti_assert!(values[0] == 11);
    }
}

#[trusted]
fn runtime_check() {
    let mut values = [1, 2, 3];
    visit(&mut values);
    assert_eq!(values, [11, 7, 7]);
    visit(&mut []);
}
fn main() { runtime_check(); }
