use prusti_contracts::*;
#[path = "../support/mut_region_pledged.rs"]
mod model;
use model::Iter;

#[requires(values.len() == 1)]
fn visit(values: &mut [u32]) {
    let mut it = Iter { tail: Some(values) };
    let item = it.next();
    match item {
        Some(item) => *item = 7,
        None => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values[0] == 7);
}

#[trusted]
fn runtime_check() {
    let mut values = [1];
    visit(&mut values);
    assert_eq!(values, [7]);
}
fn main() { runtime_check(); }
