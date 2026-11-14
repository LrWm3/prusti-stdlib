use prusti_contracts::*;

#[requires(before_expiry(*value) == 1)] // expected-unsupported: cannot encode precondition: before_expiry is only supported in expiry pledges
fn wrong(value: &mut u32) { *value = 7; }

fn main() {}
