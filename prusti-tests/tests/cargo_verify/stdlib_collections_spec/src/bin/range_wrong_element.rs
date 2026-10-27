use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let values = make(8);
    let part = &values[2..5];
    prusti_assert!(part[0].value == 3); // expected-error: assertion might not hold
}
