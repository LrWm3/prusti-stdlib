use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut values = [1, 2, 3, 4, 5];
    values.swap(0, 5); // expected-error: precondition might not hold
}
