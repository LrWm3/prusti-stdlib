use prusti_contracts::*;
#[ensures(result === old(value))]
#[ensures(resource_state(&result) == old(resource_state(&value)))]
#[after_expiry(resource_state(&value) == before_expiry(resource_state(&result)))]
fn identity<T>(value: T) -> T {
    value
}
#[requires(*first == 1 && *second == 2)]
#[ensures(*first == 7 && *second == 8)]
fn visit(first: &mut u32, second: &mut u32) {
    let output = identity([first, second]);
    let initial_first = *output[0];
    let initial_second = *output[1];
    prusti_assert!(initial_first == 1 && initial_second == 2);
    *output[0] = 7;
    *output[1] = 8;
    let first_value = *output[0];
    let second_value = *output[1];
    prusti_assert!(first_value == 7);
    prusti_assert!(second_value == 8);
}
#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    visit(&mut first, &mut second);
    assert_eq!((first, second), (7, 8));
}
