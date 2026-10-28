use prusti_contracts::*;

#[requires(values.len() == 2)]
fn observe<T>(values: &mut [T]) {
    prusti_assert!(mut_slice_len(&values) == 2);
}

#[trusted]
fn runtime_check() {
    let mut values = [1u32, 2];
    observe(&mut values);
    let mut empty_elements = [(), ()];
    observe(&mut empty_elements);
    assert_eq!(mut_slice_len(&&mut values[..]), 2);
    assert_eq!(mut_slice_len(&&mut empty_elements[..]), 2);
}
fn main() { runtime_check(); }
