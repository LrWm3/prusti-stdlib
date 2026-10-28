use prusti_contracts::*;

struct Iter<'a> { tail: Option<&'a mut [u32]> }
impl<'a> Iter<'a> {
    #[after_expiry(before_expiry(match &self.tail { Some(slice) => mut_slice_contents(slice), None => Seq::new() }) == Seq::new())] // expected-unsupported: cannot encode pledge rhs: before_expiry with branching expressions is not supported
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

fn main() {}
