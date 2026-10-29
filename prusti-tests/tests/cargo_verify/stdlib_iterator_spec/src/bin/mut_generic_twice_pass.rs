use prusti_contracts::*;
#[ensures(result === old(value))]
#[ensures(resource_state(&result) === old(resource_state(&value)))]
#[after_expiry(resource_state(&value) === before_expiry(resource_state(&result)))]
fn identity<T>(value: T) -> T {
    value
}
#[ensures(result === old(value))]
#[ensures(resource_state(&result) === old(resource_state(&value)))]
#[after_expiry(resource_state(&value) === before_expiry(resource_state(&result)))]
fn twice<T>(value: T) -> T {
    identity(identity(value))
}
fn visit(value: &mut u32) {
    let output = twice(value);
    *output = 7;
    prusti_assert!(*output == 7);
}
#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut value);
    assert_eq!(value, 7);
}
