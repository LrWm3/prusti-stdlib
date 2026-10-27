use iterator_specs::count_shared;
use prusti_contracts::*;

fn wrong<T>(values: &[T]) {
    let expected = values.len();
    let count = count_shared(values, expected);
    prusti_assert!(count != expected); // expected-error: assertion might not hold
}

fn main() {}
