extern crate stdlib_specs;

use prusti_contracts::*;
use target_api::{Identity, Owned};

fn main() {
    let value = Owned::<u32>::identity(7);
    prusti_assert!(value == 8); // expected-error: assertion might not hold
}
