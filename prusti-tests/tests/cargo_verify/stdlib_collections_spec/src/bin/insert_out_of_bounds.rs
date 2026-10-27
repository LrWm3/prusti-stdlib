use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut values = make(8);
    values.insert(9, collection_specs::Owned { value: 19 }); // expected-error: precondition might not hold
}
