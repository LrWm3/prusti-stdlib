use prusti_contracts::*;
#[after_expiry(*value == old(*value))] // expected-error: pledge postcondition might not hold
fn borrow(value: &mut u32) -> &mut u32 { value }
fn main() {}
