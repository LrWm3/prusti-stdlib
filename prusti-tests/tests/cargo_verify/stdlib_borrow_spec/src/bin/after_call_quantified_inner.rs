use prusti_contracts::*;

#[after_expiry(forall(|j: Int| after_call(*value) == 7))] // expected-unsupported: cannot encode pledge rhs: after_call is only supported in expiry pledges outside other temporal expressions
fn borrow(value: &mut u32) -> &mut u32 { *value = 7; value }
fn main() {}
