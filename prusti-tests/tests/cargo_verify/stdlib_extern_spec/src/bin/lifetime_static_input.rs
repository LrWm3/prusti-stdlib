use prusti_contracts::*;

#[extern_spec(target_api)] // expected-incorrect: Invalid external specification
#[trusted]
fn shared(value: &'static u32) -> &'static u32;
// expected-note: Lifetime relationships must match method 'target_api::shared' exactly

fn main() {}
