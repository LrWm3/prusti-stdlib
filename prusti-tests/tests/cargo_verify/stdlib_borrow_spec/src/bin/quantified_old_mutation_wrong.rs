use prusti_contracts::*;

#[requires(values.len() == 1 && values[0] == 1)]
#[ensures(forall(|i: usize| i < values.len() ==> values[i] == old(values[i])))] // expected-error: postcondition might not hold
fn update(values: &mut [u32]) { values[0] = 7; }

fn main() {}
