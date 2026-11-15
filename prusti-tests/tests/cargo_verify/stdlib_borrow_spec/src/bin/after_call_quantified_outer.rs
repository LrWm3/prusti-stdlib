use prusti_contracts::*;

#[after_expiry(after_call(forall(|j: Int| *value == 7)))] // expected-unsupported: cannot encode pledge rhs: specification closures inside after_call are not supported
fn borrow(value: &mut u32) -> &mut u32 { *value = 7; value }
fn main() {}
