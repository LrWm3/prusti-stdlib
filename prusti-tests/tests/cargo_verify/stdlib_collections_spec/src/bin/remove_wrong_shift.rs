use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut values = make(8);
    values.remove(3);
    prusti_assert!(values[3].value == 3); // expected-error: assertion might not hold
}
