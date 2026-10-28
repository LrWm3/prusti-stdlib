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
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {}
