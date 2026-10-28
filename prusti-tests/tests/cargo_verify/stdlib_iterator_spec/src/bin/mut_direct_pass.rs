use prusti_contracts::*;

#[requires(values.len() == 2)]
#[requires(values[0] == 1 && values[1] == 2)]
fn visit(values: &mut [u32]) {
    let mut iterator = values.iter_mut();
    prusti_assert!(mut_slice_len(&*mut_iter_tail(&iterator)) == 2);
    match iterator.next() {
        Some(value) => { prusti_assert!(*value == 1); *value = 7; }
        None => { prusti_assert!(false); }
    }
    prusti_assert!(mut_slice_len(&*mut_iter_tail(&iterator)) == 1);
    match iterator.next() {
        Some(value) => { prusti_assert!(*value == 2); *value = 8; }
        None => { prusti_assert!(false); }
    }
    let next = iterator.next();
    prusti_assert!(matches!(next, None));
}

#[trusted]
fn runtime_check() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 8]);
}
fn main() { runtime_check(); }
