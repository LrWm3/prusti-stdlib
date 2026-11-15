use prusti_contracts::*;
fn main() {
    let mut left = 4u32;
    let mut right = 11u32;
    let mut replacement = 15u32;
    let mut values = [&mut left, &mut right];
    let previous = core::mem::replace(&mut values[0], &mut replacement);
    *previous = 7;
    *values[0] = 9;
    prusti_assert!(left == 9); // expected-error: assertion might not hold
    prusti_assert!(right == 11);
    prusti_assert!(replacement == 9);
}
