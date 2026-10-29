use prusti_contracts::*;
#[requires(values.len() > 0)]
fn head<T>(values: &mut [T]) -> &mut T {
    let first = &mut values[0];
    prusti_assert!(false); // expected-error: assertion might not hold
    first
}
#[trusted]
fn main() {}
