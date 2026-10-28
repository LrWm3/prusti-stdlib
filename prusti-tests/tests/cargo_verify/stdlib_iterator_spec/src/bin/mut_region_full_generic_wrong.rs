use prusti_contracts::*;
#[path = "../support/mut_region_full.rs"]
mod model;
use model::Iter;

struct Token { value: u32 }

#[requires(values.len() == 2)]
fn visit(values: &mut [Token]) {
    let mut it = Iter { tail: Some(values) };
    let first = it.next();
    let second = it.next();
    match (first, second) {
        (Some(first), Some(second)) => { first.value = 7; second.value = 8; }
        _ => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values[1].value == 9); // expected-error: assertion might not hold
}

fn main() {}
