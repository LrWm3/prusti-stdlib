use prusti_contracts::*;
#[ensures(result === old(value))]
#[ensures(resource_state(&result) === old(resource_state(&value)))]
#[after_expiry(resource_state(&value) === before_expiry(resource_state(&result)))]
fn identity<T>(value: T) -> T {
    value
}
#[requires(**value == 1)]
#[ensures(Ghost::new_ref(&*value) == old(Ghost::new_ref(&*value)))]
#[ensures(**value == 7)]
fn visit(value: &mut &mut u32) {
    let output = identity(value);
    prusti_assert!(**output == 1);
    **output = 7;
    prusti_assert!(**output == 7);
    prusti_assert!(false); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut &mut value);
    assert_eq!(value, 7);
}
