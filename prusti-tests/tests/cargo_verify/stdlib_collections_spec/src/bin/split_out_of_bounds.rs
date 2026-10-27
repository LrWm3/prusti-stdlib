use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let values = make(8);
    let _parts = values.split_at(9); // expected-error: precondition might not hold
}
