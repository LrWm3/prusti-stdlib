use prusti_contracts::*;

struct Iter<'a> { tail: &'a mut [u32] }
impl<'a> Iter<'a> {
    // No postcondition says that this returns None, so callers must handle
    // either result and retain the corresponding lifetime resources.
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

struct OptionalIter<'a> { tail: Option<&'a mut [u32]> }
impl<'a> OptionalIter<'a> {
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

#[requires(values.len() == 2)]
fn first_expires_first(values: &mut [u32]) {
    let mut iterator = Iter { tail: values };
    let first = iterator.next();
    let second = iterator.next();
    if let (Some(first), Some(second)) = (first, second) { *first = 7; *second = 8; }
    drop(iterator);
    prusti_assert!(values.len() == 2);
}

#[requires(values.len() == 2)]
fn second_expires_first(values: &mut [u32]) {
    let mut iterator = Iter { tail: values };
    let first = iterator.next();
    let second = iterator.next();
    if let (Some(first), Some(second)) = (first, second) { *second = 8; *first = 7; }
    drop(iterator);
    prusti_assert!(values.len() == 2);
}

#[requires(values.len() == 2)]
fn optional_tail(values: &mut [u32]) {
    let mut iterator = OptionalIter { tail: Some(values) };
    let first = iterator.next();
    let second = iterator.next();
    if let (Some(first), Some(second)) = (first, second) { *first = 7; *second = 8; }
    drop(iterator);
    prusti_assert!(values.len() == 2);
}

fn optional_empty() {
    let mut iterator = OptionalIter { tail: None };
    let first = iterator.next();
    let second = iterator.next();
    if let (Some(first), Some(second)) = (first, second) { *first = 7; *second = 8; }
}

#[trusted]
fn runtime_check() {
    let mut values = [1, 2];
    first_expires_first(&mut values);
    second_expires_first(&mut values);
    optional_tail(&mut values);
    optional_empty();
    assert_eq!(values, [1, 2]);
}
fn main() { runtime_check(); }
