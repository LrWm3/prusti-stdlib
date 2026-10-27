use prusti_contracts::*;
use stdlib_specs::conditional_static_ref;

fn arbitrary<'a, T>(value: &'a T) {
    let result = conditional_static_ref(&value);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
