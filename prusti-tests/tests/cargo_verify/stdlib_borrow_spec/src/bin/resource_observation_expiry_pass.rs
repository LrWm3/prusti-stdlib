use prusti_contracts::*;
#[after_expiry(resource_state(&value) == before_expiry(resource_state(&result)))]
fn borrow(value: &mut u32) -> &mut u32 { value }
#[requires(*value == 4)]
fn visit(value: &mut u32) {
    let entry = ghost! { resource_state(&value) };
    let returned = borrow(value);
    *returned = 7;
    prusti_assert!(*entry != resource_state(&value));
    prusti_assert!(*value == 7);
}
fn main() { let mut value = 4; visit(&mut value); }
