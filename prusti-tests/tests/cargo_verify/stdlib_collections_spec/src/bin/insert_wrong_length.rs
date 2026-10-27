use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut values = make(8);
    values.insert(3, collection_specs::Owned { value: 29 });
    prusti_assert!(values.len() == 8); // expected-error: assertion might not hold
}
