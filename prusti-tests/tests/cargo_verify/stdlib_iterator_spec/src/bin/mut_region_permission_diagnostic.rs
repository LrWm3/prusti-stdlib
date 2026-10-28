use prusti_contracts::*;

struct Iter<'a> { tail: &'a mut [u32] }
impl<'a> Iter<'a> {
    // A deliberately weak contract keeps the first result unconstrained.
    #[trusted]
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

// This valid program currently exposes the missing permission split. This
// case tests the diagnostic; it does not establish that the program is wrong.
fn diagnostic(values: &mut [u32]) {
    let mut iterator = Iter { tail: values };
    let first = iterator.next();
    let second = iterator.next(); // expected-error: insufficient permission to call this function
    if let (Some(first), Some(second)) = (first, second) { *first = 7; *second = 8; }
}
fn main() {}
