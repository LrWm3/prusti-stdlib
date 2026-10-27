use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let values = make(8);
    let _part = &values[5..3]; // expected-error: precondition might not hold
}
