use prusti_contracts::*;

// The call in the generated stub compiles by coercing &mut u32 to &u32,
// but the external contract must describe the target's actual signature.
#[extern_spec(target_api)] // expected-incorrect: Invalid external specification
#[trusted]
fn shared(value: &mut u32) -> &u32;
// expected-note: Parameter and return types must match method 'target_api::shared' exactly

fn main() {}
