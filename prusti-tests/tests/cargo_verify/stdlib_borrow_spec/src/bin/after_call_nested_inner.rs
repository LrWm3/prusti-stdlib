use prusti_contracts::*;

struct Holder<'a> { value: &'a mut u32, count: u32 }
impl<'a> Holder<'a> {
    #[after_expiry(after_call(old(self.count)) == 7)] // expected-unsupported: cannot encode pledge rhs: after_call is only supported in expiry pledges outside other temporal expressions
    fn next(&mut self) -> Option<&'a mut u32> { self.count = 7; None }
}

fn main() {}
