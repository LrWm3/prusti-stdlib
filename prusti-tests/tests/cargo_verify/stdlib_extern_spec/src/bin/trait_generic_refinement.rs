extern crate stdlib_specs;

use prusti_contracts::*;
use target_api::Bounded;

// Zero's implementation contract must not leak into arbitrary Bounded calls.
#[requires(value <= 10)]
#[ensures(result == 0)] // expected-error: postcondition might not hold
pub fn generic_bound<B: Bounded>(value: u32) -> u32 {
    B::bounded(value)
}

fn main() {}
