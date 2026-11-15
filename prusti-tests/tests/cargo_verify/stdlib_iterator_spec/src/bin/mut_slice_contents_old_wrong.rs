use prusti_contracts::*;

#[requires(values.len() == 1)]
#[requires(values[0] == 1)]
#[ensures(old(mut_slice_contents(&values))[0] == Ghost::new_ref(&7))] // expected-error: postcondition might not hold
fn wrong(values: &mut [u32]) { values[0] = 7; }

fn main() {}
