use prusti_contracts::*;
#[path = "../support/mut_region_pledged.rs"]
mod model;
use model::Iter;

#[requires(values.len() == 1)]
fn wrong(values: &mut [u32]) {
    let mut it = Iter { tail: Some(values) };
    let item = it.next();
    match item {
        Some(item) => *item = 7,
        None => { prusti_assert!(false); }
    }
    drop(it);
    prusti_assert!(values[0] == 8); // expected-error: assertion might not hold
}

fn main() {}
