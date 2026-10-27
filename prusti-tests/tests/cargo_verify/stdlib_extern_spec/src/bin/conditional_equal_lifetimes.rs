use prusti_contracts::*;
use stdlib_specs::conditional_same_lifetime;

fn arbitrary<'a, 'b>(value: &(&'a u8, &'b u8)) {
    let result = conditional_same_lifetime(value);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
