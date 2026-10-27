use prusti_contracts::*;
use stdlib_specs::conditional_copy_ref;

fn unbounded<'a, T>(value: &'a T) {
    let result = conditional_copy_ref(&value);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
