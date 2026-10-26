extern crate stdlib_specs;

use prusti_contracts::*;
use target_api::Bounded;

struct Local;

#[refine_trait_spec]
impl Bounded for Local {
    #[requires(value <= 10)]
    #[ensures(result <= 11)] // expected-error: the implementation's postcondition may be weaker than the trait method's
    fn bounded(value: u32) -> u32 {
        0
    }
}

fn main() {}
