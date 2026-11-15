use prusti_contracts::*;
fn main() {
    let mut left = 4u32;
    let mut right = 11u32;
    let mut stored = &mut left;
    let previous = core::mem::replace(&mut stored, &mut right);
    *previous = 7;
    *stored = 9;
    prusti_assert!(left == 7);
    prusti_assert!(right == 9);
}
