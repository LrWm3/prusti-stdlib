use prusti_contracts::*;
fn main() {
    let mut value = 4u32;
    let mut inner = &mut value;
    let nested = &mut inner;
    let entry = ghost! { resource_state(&nested) };
    **nested = 7;
    prusti_assert!(*entry == resource_state(&nested)); // expected-error: assertion might not hold
    prusti_assert!(value == 7);
}
