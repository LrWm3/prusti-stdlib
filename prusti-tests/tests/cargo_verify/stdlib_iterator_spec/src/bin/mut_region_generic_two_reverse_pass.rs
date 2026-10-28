use prusti_contracts::*;
#[path = "../support/mut_region_generic.rs"]
mod model;
use model::Iter;

struct Token { value: u32 }

#[requires(values.len() == 2)]
fn visit(values: &mut [Token]) {
    let mut it = Iter { tail: Some(values) };
    let first = it.next();
    let second = it.next();
    match (first, second) {
        (Some(first), Some(second)) => { second.value = 8; first.value = 7; }
        _ => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values[0].value == 7);
    prusti_assert!(values[1].value == 8);
}

#[trusted]
fn runtime_check() {
    let mut values = [Token { value: 1 }, Token { value: 2 }];
    visit(&mut values);
    assert_eq!(values[0].value, 7);
    assert_eq!(values[1].value, 8);
}
fn main() { runtime_check(); }
