extern crate stdlib_specs;

use prusti_contracts::*;

#[extern_spec(target_api)]
trait Bounded {
    #[trusted]
    fn bounded(value: u32) -> u32; // expected-incorrect: duplicate specification for `target_api::Bounded::bounded` from crate `duplicate_imported_trait` and `stdlib_specs`
}

fn main() {}
