use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut values = make(8);
    values.swap_remove(8); // expected-error: precondition might not hold
}
