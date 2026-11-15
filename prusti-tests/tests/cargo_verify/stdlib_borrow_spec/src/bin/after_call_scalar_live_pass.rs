use prusti_contracts::*;

#[ensures(Ghost::new_ref(&result) == old(Ghost::new_ref(&value)))]
#[after_expiry(after_call(*value) == 7 && *value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 { *value = 7; value }

#[ensures(*value == 9)]
fn visit(value: &mut u32) {
    let loan = borrow(value);
    *loan = 9;
    prusti_assert!(*value == 9);
}
fn main() { let mut value = 1; visit(&mut value); assert_eq!(value, 9); }
