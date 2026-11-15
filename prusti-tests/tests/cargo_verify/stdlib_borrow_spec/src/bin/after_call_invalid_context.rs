use prusti_contracts::*;

#[ensures(after_call(*value) == 7)] // expected-unsupported: cannot encode postcondition: after_call is only supported in expiry pledges outside other temporal expressions
fn wrong(value: &mut u32) { *value = 7; }

fn main() {}
