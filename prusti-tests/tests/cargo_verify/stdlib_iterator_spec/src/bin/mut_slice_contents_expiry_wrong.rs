use prusti_contracts::*;

#[requires(values.len() == 1)]
#[requires(values[0] == 1)]
#[after_expiry(before_expiry(mut_slice_contents(&result))[0] == Ghost::new_ref(&1))] // expected-error: pledge postcondition might not hold
fn wrong(values: &mut [u32]) -> &mut [u32] { values }

fn main() {}
