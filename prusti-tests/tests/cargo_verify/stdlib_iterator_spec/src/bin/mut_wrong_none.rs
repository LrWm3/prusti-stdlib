use prusti_contracts::*;

#[requires(values.len() == 1)]
fn wrong(values: &mut [u32]) {
    let mut iterator = values.iter_mut();
    prusti_assert!(mut_slice_len(&*mut_iter_tail(&iterator)) == 1);
    let next = iterator.next();
    prusti_assert!(matches!(next, None)); // expected-error: assertion might not hold
}
fn main() {}
