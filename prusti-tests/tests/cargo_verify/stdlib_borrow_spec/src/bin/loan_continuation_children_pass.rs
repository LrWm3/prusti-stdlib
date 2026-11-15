use prusti_contracts::*;
#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 { value }
fn main() {
    let mut value = 4u32;
    let parent = borrow(&mut value);
    let child = borrow(&mut *parent);
    *child = 7;
    let child_final = *child;
    *parent = 9;
    prusti_assert!(value == 9);
    prusti_assert!(child_final == 7);
}
