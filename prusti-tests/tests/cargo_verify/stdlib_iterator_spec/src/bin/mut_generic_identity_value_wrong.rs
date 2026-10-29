use prusti_contracts::*;
#[ensures(result === old(value))]
fn identity<T>(value: T) -> T {
    value
}
fn visit(value: &mut u32) {
    let output = identity(value);
    *output = 7;
    prusti_assert!(*output == 8); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut value);
    assert_eq!(value, 7);
}
