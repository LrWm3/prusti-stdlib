use prusti_contracts::*;
#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 { value }
fn main() {
    let mut left = 4u32;
    let mut right = 11u32;
    let mut returned = borrow(&mut left);
    *returned = 7;
    returned = borrow(&mut right);
    *returned = 9;
    prusti_assert!(left == 7);
    prusti_assert!(right == 9);
}
