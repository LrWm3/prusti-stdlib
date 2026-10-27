use prusti_contracts::*;
use stdlib_specs::conditional_observe;

fn arbitrary_lifetime<'a>(value: &'a u8) {
    let result = conditional_observe(&value);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
