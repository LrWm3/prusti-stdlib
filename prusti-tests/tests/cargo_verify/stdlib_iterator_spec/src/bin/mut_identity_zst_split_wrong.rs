use prusti_contracts::*;

#[requires(values.len() == 2)]
fn wrong(values: &mut [()]) {
    let (left, right) = values.split_at_mut(1);
    let first = &mut left[0];
    let second = &mut right[0];
    *first = ();
    *second = ();
    prusti_assert!(!mut_ref_same_address(&first, &second)); // expected-error: assertion might not hold
}
fn main() {}
