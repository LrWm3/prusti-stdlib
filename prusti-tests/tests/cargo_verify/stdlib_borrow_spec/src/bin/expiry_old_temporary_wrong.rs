use prusti_contracts::*;

#[requires(values.len() == 1)]
#[ensures(result.len() == old(values.len()))]
#[after_expiry(values => { // expected-error: pledge postcondition might not hold
    let source = old(mut_slice_contents(&values));
    Seq::from_slice(&*values).len() == source.len() + Int::from(1)
})]
fn relay(values: &mut [u32]) -> &mut [u32] { values }

fn main() {}
