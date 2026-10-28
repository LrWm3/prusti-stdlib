use iterator_specs::prefix_owned;
use prusti_contracts::*;

#[requires(expected == values.len())]
#[requires(stop_after > 0 && stop_after < expected)]
fn wrong<T>(values: Vec<T>, expected: usize, stop_after: usize) {
    let count = prefix_owned(values, expected, stop_after);
    prusti_assert!(count == expected); // expected-error: assertion might not hold
}
fn main() {}
