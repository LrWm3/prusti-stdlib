use prusti_contracts::*;
#[path = "../support/mut_region_generic.rs"]
mod model;
use model::Iter;

struct Token { value: u32 }

#[requires(values.len() == 1)]
fn wrong(values: &mut [Token]) {
    let mut it = Iter { tail: Some(values) };
    let first = it.next();
    match first { Some(first) => first.value = 7, None => { prusti_assert!(false); } }
    drop(it);
    prusti_assert!(values[0].value == 8); // expected-error: assertion might not hold
}

fn main() {}
