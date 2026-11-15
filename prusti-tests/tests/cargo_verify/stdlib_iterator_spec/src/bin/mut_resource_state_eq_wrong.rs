use prusti_contracts::*;

#[requires(*first == 1 && *second == 2)]
fn compare(first: &mut u32, second: &mut u32) {
    prusti_assert!(resource_state(&first) == resource_state(&second)); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let (mut first, mut second) = (1, 2);
    compare(&mut first, &mut second);
}
