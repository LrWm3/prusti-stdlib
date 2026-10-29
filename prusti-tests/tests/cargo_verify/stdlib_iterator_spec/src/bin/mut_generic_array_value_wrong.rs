use prusti_contracts::*;
#[ensures(result === old(value))]
fn identity<T>(value: T) -> T {
    value
}
fn visit(first: &mut u32, second: &mut u32) {
    let output = identity([first, second]);
    *output[0] = 7;
    *output[1] = 8;
    let first_value = *output[0];
    let second_value = *output[1];
    prusti_assert!(first_value == 7);
    prusti_assert!(second_value == 7); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut first, &mut second);
    assert_eq!((first, second), (7, 8));
}
