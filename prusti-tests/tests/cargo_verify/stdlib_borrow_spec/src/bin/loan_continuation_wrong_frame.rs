use prusti_contracts::*;
#[ensures(*result == old(*value))]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &mut u32) -> &mut u32 { value }
fn main() {
    let mut value = 4u32;
    let untouched = 11u32;
    let returned = borrow(&mut value);
    *returned = 7;
    prusti_assert!(value == 7);
    prusti_assert!(untouched == 7); // expected-error: assertion might not hold
}
