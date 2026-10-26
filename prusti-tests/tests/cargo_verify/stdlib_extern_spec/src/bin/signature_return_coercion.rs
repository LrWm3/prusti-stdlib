use prusti_contracts::*;

#[extern_spec(target_api)] // expected-incorrect: Invalid external specification
#[trusted]
fn mutable(value: &mut u32) -> &u32;
// expected-note: Parameter and return types must match method 'target_api::mutable' exactly

fn main() {}
