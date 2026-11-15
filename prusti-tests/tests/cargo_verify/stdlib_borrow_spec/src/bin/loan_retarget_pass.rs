use prusti_contracts::*;
fn main() {
    let mut left = 4u32;
    let mut right = 11u32;
    let mut loan = &mut left;
    *loan = 7;
    loan = &mut right;
    *loan = 9;
    prusti_assert!(left == 7);
    prusti_assert!(right == 9);
}
