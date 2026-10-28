use prusti_contracts::*;

#[requires(values.len() == 1 && values[0] == 1)]
fn wrong(values: &mut [u32]) {
    let mut iterator = values.iter_mut();
    prusti_assert!(mut_slice_len(&*mut_iter_tail(&iterator)) == 1);
    if let Some(value) = iterator.next() { *value = 7; }
    prusti_assert!(values[0] == 1); // expected-error: assertion might not hold
}
fn main() {}
