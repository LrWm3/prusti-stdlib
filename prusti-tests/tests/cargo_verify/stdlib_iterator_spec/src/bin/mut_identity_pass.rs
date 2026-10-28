use iterator_specs::{mut_remaining, mut_remaining_refs};
use prusti_contracts::*;

fn shared_reborrow(value: &mut u32) {
    let view = &*value;
    prusti_assert!(mut_ref_same_address(&value, &value));
    prusti_assert!(mut_ref_shared_same_address(&value, view));
}

#[requires(values.len() == 2)]
fn visit(values: &mut [u32]) {
    let mut iterator = values.iter_mut();
    let references = ghost! { mut_remaining_refs(&iterator) };
    prusti_assert!(mut_remaining(&iterator).len() == Int::from(2));
    let first = iterator.next();
    let second = iterator.next();
    match (first, second) {
        (Some(first), Some(second)) => {
            prusti_assert!(mut_ref_same_address(&first, &*(*references)[0]));
            prusti_assert!(mut_ref_same_address(&second, &*(*references)[1]));
            *first = 7;
            *second = 8;
        }
        _ => { prusti_assert!(false); }
    }
}

#[trusted]
fn runtime_check() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 8]);
    shared_reborrow(&mut values[0]);
    let mut empty = [(), ()];
    let (left, right) = empty.split_at_mut(1);
    let left = &mut left[0];
    let right = &mut right[0];
    assert!(mut_ref_same_address(&left, &right));
}
fn main() { runtime_check(); }
