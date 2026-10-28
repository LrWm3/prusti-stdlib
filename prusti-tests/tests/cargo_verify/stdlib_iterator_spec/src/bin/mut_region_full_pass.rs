use prusti_contracts::*;
#[path = "../support/mut_region_full.rs"]
mod model;
use model::Iter;

#[requires(values.len() == 2)]
fn visit(values: &mut [u32]) {
    let mut it = Iter { tail: Some(values) };
    let first = it.next();
    let second = it.next();
    match (first, second) {
        (Some(first), Some(second)) => { *first = 7; *second = 8; }
        _ => { prusti_assert!(false); }
    }
    match it.next() {
        None => (),
        Some(_) => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values[0] == 7);
    prusti_assert!(values[1] == 8);
}

#[trusted]
fn runtime_check() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 8]);
}
fn main() { runtime_check(); }
