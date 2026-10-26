extern crate stdlib_specs;
use prusti_contracts::*;

pub struct Token {
    value: u32,
}

// Shared references carry no indirect permissions, but their pledges must remain.
#[after_expiry(token.value == 42)] // expected-error: pledge postcondition might not hold
pub fn project(token: &Token) -> &u32 {
    &token.value
}

fn main() {}
