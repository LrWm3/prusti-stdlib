use collection_specs::{make, view};
use prusti_contracts::*;

fn main() {
    let mut values = make(8);
    let before = ghost! { view(&values) };
    values.clear();
    prusti_assert!(view(&values) == *before); // expected-error: assertion might not hold
}
