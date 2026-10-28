use prusti_contracts::*;

#[requires(values.len() == 2)]
fn wrong<T>(values: &mut [T]) {
    prusti_assert!(mut_slice_len(&values) == 3); // expected-error: assertion might not hold
}

fn main() {}
