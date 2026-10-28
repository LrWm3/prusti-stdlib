use prusti_contracts::*;
#[path = "../support/mut_region_reassembly.rs"]
mod model;
use model::Iter;

#[requires(values.len() == 2)]
fn wrong(values: &mut [u32]) {
    let mut it = Iter { tail: Some(values) };
    let first = it.next();
    let second = it.next();
    match (first, second) {
        (Some(first), Some(second)) => { *first = 7; *second = 8; }
        _ => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {}
