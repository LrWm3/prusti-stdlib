use prusti_contracts::*;

#[requires(values.len() == 2)]
fn wrong(values: &mut [()]) {
    let mut it = values.iter_mut();
    let first = it.next();
    let second = it.next();
    if let (Some(first), Some(second)) = (first, second) {
        prusti_assert!(!mut_ref_same_address(&first, &second)); // expected-error: assertion might not hold
    }
}
fn main() {}
