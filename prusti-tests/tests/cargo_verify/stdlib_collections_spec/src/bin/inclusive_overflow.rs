use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let values = make(8);
    let _part = &values[0..=usize::MAX]; // expected-error: precondition might not hold
}
