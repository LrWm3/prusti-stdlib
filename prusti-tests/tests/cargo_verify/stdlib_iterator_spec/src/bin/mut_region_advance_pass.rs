use prusti_contracts::*;

struct Iter<'a> { tail: Option<&'a mut [u32]> }
impl<'a> Iter<'a> {
    fn next(&mut self) -> Option<&'a mut u32> {
        let slice = match core::mem::replace(&mut self.tail, None) {
            Some(slice) => slice,
            None => return None,
        };
        if slice.is_empty() { self.tail = Some(slice); return None; }
        let (first, rest) = slice.split_at_mut(1);
        self.tail = Some(rest);
        Some(&mut first[0])
    }
}

#[requires(values.len() == 2)]
fn visit(values: &mut [u32]) {
    let mut iterator = Iter { tail: Some(values) };
    let first = iterator.next();
    let second = iterator.next();
    if let (Some(first), Some(second)) = (first, second) { *first = 7; *second = 8; }
    drop(iterator);
    prusti_assert!(values.len() == 2);
}

#[trusted]
fn runtime_check() {
    let mut values = [1, 2];
    visit(&mut values);
    assert_eq!(values, [7, 8]);
}
fn main() { runtime_check(); }
