use prusti_contracts::*;

#[ensures(resource_state(&value) == old(resource_state(&value)))] // expected-error: postcondition might not hold
fn discard<T>(value: T) {
    drop(value);
}

fn main() {}
