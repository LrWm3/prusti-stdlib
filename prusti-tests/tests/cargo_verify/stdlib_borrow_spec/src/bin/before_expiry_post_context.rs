use prusti_contracts::*;

#[ensures(before_expiry(*value) == 7)] // expected-unsupported: cannot encode postcondition: before_expiry is only supported in expiry pledges
fn wrong(value: &mut u32) { *value = 7; }

fn main() {}
