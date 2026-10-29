use prusti_contracts::*;
#[ensures(resource_state(&value) === old(resource_state(&value)))] // expected-error: postcondition might not hold
fn change(value: &mut u32) {
    *value = 9;
}
#[trusted]
fn main() {}
