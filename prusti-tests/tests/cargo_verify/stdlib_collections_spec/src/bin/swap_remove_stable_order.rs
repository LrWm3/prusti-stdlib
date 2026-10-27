use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut values = make(8);
    values.swap_remove(1);
    prusti_assert!(values[1].value == 2); // expected-error: assertion might not hold
}
