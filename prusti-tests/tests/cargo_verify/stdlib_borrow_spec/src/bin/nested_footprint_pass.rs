use prusti_contracts::*;

struct Holder<'a> { value: &'a mut u32, count: u32 }
impl<'a> Holder<'a> {
    // No pledge: checking the structural ownership contract must be sufficient.
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

struct OptionalHolder<'a> { value: Option<&'a mut u32> }
impl<'a> OptionalHolder<'a> {
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

#[trusted]
fn runtime_check() {
    let mut value = 3;
    let mut holder = Holder { value: &mut value, count: 1 };
    assert!(holder.next().is_none());
    assert_eq!(holder.count, 1);
    assert_eq!(*holder.value, 3);
    let mut empty = OptionalHolder { value: None };
    assert!(empty.next().is_none());
}
fn main() { runtime_check(); }
