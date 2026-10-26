extern crate stdlib_specs;

use prusti_contracts::*;
use target_api::Bounded;

struct Local;

#[refine_trait_spec]
impl Bounded for Local {
    #[requires(value < 10)] // expected-error: the implementation's precondition may be stronger than the trait method's
    #[ensures(result == 0)]
    fn bounded(value: u32) -> u32 {
        0
    }
}

fn main() {}
