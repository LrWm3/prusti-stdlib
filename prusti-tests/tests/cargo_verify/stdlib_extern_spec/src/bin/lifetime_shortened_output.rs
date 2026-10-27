use prusti_contracts::*;

#[extern_spec(target_api)] // expected-incorrect: Invalid external specification
#[trusted]
fn static_ref(value: &u32) -> &u32;
// expected-note: Lifetime relationships must match method 'target_api::static_ref' exactly

fn main() {}
