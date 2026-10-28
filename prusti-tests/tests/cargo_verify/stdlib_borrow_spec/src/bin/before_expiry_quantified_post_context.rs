use prusti_contracts::*;

#[ensures(forall(|i: usize| i < 1 ==> before_expiry(*value) == 7))] // expected-unsupported: cannot encode postcondition: before_expiry is only supported in expiry pledges
fn wrong(value: &mut u32) { *value = 7; }

fn main() {}
