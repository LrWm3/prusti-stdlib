use prusti_contracts::*;
#[after_expiry(true)]
fn separate<'a, 'b>(first: &'a mut u32, second: &'b mut u32) -> (&'a mut u32, &'b mut u32) { // expected-unsupported: cannot encode method `separate`: pledges on a function whose result contains borrows that expire separately require an explicit input expiry selector
    (first, second)
}
fn main() {}
