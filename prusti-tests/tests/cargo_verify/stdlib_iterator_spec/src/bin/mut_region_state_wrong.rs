use prusti_contracts::*;

struct Iter<'a> { tail: &'a mut [u32] }
impl<'a> Iter<'a> {
    #[trusted]
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

fn wrong(values: &mut [u32]) {
    let mut iterator = Iter { tail: values };
    let first = iterator.next();
    let second = iterator.next();
    if let (Some(first), Some(second)) = (first, second) { *first = 7; *second = 8; }
    drop(iterator);
    prusti_assert!(false); // expected-error: assertion might not hold
}
fn main() {}
