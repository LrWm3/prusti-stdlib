use prusti_contracts::*;

#[ensures(Ghost::new_ref(&result) == old(Ghost::new_ref(&value)))]
#[after_expiry(after_call(*value) == 8)] // expected-error: pledge postcondition might not hold
fn borrow(value: &mut u32) -> &mut u32 { *value = 7; value }
fn main() {}
