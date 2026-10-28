use prusti_contracts::*;
#[path = "../support/mut_region_generic.rs"]
mod model;
use model::Iter;

struct Token { value: u32 }

#[requires(values.len() == 1)]
fn visit(values: &mut [Token]) {
    let mut it = Iter { tail: Some(values) };
    let first = it.next();
    match first { Some(first) => first.value = 7, None => { prusti_assert!(false); } }
    drop(it);
    prusti_assert!(values[0].value == 7);
}

#[trusted]
fn runtime_check() {
    let mut values = [Token { value: 1 }];
    visit(&mut values);
    assert_eq!(values[0].value, 7);
}
fn main() { runtime_check(); }
