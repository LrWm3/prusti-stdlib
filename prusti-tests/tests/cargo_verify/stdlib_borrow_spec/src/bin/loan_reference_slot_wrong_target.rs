use prusti_contracts::*;
fn main() {
    let mut left = 4u32;
    let mut right = 11u32;
    let mut stored = &mut left;
    let slot = &mut stored;
    **slot = 7;
    *slot = &mut right;
    **slot = 9;
    prusti_assert!(left == 9); // expected-error: assertion might not hold
}
