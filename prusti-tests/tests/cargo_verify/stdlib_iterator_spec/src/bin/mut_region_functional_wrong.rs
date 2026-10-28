use prusti_contracts::*;
#[path = "../support/mut_region_functional.rs"]
mod model;
use model::{Iter, remaining};

#[requires(values.len() == 2)]
fn wrong(values: &mut [u32]) {
    let mut it = Iter { tail: Some(values) };
    let first = it.next();
    let second = it.next();
    prusti_assert!(remaining(&it) == 0);
    match (first, second) {
        (Some(first), Some(second)) => {
            *first = 7;
            *second = 8;
            prusti_assert!(false); // expected-error: assertion might not hold
        }
        _ => { prusti_assert!(false); }
    }
}
fn main() {}
