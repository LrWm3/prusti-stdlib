use prusti_contracts::*;
#[requires(*value == 4)]
fn visit(value: &mut u32) {
    let entry = ghost! { resource_state(&value) };
    *value = 7;
    let middle = ghost! { resource_state(&value) };
    *value = 9;
    prusti_assert!(*entry != *middle);
    prusti_assert!(*middle == resource_state(&value)); // expected-error: assertion might not hold
}
fn main() { let mut value = 4; visit(&mut value); }
