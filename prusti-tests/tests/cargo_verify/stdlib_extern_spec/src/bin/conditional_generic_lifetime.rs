use prusti_contracts::*;
use stdlib_specs::conditional_static;

fn arbitrary<Value>(value: &Value) {
    let result = conditional_static(value);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
