use prusti_contracts::*;

struct Holder<'a> { value: &'a mut u32, count: u32 }
impl<'a> Holder<'a> {
    #[after_expiry(after_call(self.count) == 8)] // expected-error: pledge postcondition might not hold
    fn next(&mut self) -> Option<&'a mut u32> { self.count = 7; None }
}

fn main() {}
