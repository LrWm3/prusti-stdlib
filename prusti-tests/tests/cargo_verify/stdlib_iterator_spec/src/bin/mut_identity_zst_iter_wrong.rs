use prusti_contracts::*;

#[requires(values.len() == 2)]
fn wrong(values: &mut [()]) {
    let mut iterator = values.iter_mut();
    prusti_assert!(mut_slice_len(&*mut_iter_tail(&iterator)) == 2);
    let first = iterator.next();
    let second = iterator.next();
    if let (Some(first), Some(second)) = (first, second) {
        *first = ();
        *second = ();
        prusti_assert!(!mut_ref_same_address(&first, &second)); // expected-error: assertion might not hold
    }
}
fn main() {}
