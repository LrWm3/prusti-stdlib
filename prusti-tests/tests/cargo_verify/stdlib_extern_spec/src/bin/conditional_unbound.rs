use prusti_contracts::*;
use stdlib_specs::conditional_observe;

fn unbounded<Value>(value: &Value) {
    let result = conditional_observe(value);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
