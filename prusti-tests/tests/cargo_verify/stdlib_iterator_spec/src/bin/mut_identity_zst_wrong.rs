use prusti_contracts::*;

// Distinct references need not have distinct addresses for a zero-sized type.
fn wrong<T>(first: &mut T, second: &mut T, a: T, b: T) {
    *first = a;
    *second = b;
    prusti_assert!(!mut_ref_same_address(&first, &second)); // expected-error: assertion might not hold
}
fn main() {}
