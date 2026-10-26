extern crate stdlib_specs;
use prusti_contracts::*;

fn main() {
    prusti_assert!(false); // expected-error: assertion might not hold
}
