use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut values = make(8);
    values.truncate(10);
    prusti_assert!(values.len() == 10); // expected-error: assertion might not hold
}
