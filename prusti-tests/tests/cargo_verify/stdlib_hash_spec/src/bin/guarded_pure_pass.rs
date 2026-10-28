use key_specs::{Guarded, Identity};
use prusti_contracts::*;

#[requires(input > 0)]
fn valid(input: u32) {
    let value = Identity.apply(input);
    prusti_assert!(value == input);
}

fn main() {
    valid(1);
}
