use prusti_contracts::*;
#[after_expiry(missing => true)] // expected-parse: expiry selector must name a function input
fn identity(value: &mut u32) -> &mut u32 { value }
fn main() {}
