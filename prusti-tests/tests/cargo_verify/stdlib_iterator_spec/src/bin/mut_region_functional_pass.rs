use prusti_contracts::*;
#[path = "../support/mut_region_functional.rs"]
mod model;
use model::{Iter, remaining};

#[requires(values.len() == 2)]
fn visit(values: &mut [u32]) {
    let mut it = Iter { tail: Some(values) };
    prusti_assert!(remaining(&it) == 2);
    let first = it.next();
    prusti_assert!(remaining(&it) == 1);
    let second = it.next();
    prusti_assert!(remaining(&it) == 0);
    match (first, second) {
        (Some(first), Some(second)) => { *first = 7; *second = 8; }
        _ => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values.len() == 2);
}

#[trusted]
fn runtime_check() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 8]);
}
fn main() { runtime_check(); }
