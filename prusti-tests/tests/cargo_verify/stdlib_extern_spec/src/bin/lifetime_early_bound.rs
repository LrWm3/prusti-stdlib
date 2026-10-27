use prusti_contracts::*;

#[extern_spec(target_api)] // expected-incorrect: Invalid external specification
#[trusted]
fn outlives<'a>(first: &'a u32, second: &'a u32) -> &'a u32;
// expected-note: Lifetime relationships must match method 'target_api::outlives' exactly

fn main() {}
