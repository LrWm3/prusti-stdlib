use prusti_contracts::*;

struct Holder<'a> { value: &'a mut u32, count: u32 }
impl<'a> Holder<'a> {
    #[after_expiry(before_expiry(if self.count == 0 { 1 } else { 2 }) == 1)] // expected-error: insufficient permission to read through mutable reference
    fn next(&mut self) -> Option<&'a mut u32> { None }
}

fn main() {}
