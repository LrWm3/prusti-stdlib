use prusti_contracts::*;
use resource_specs::identity;
#[requires(**value == 1)]
#[ensures(Ghost::new_ref(&*value) == old(Ghost::new_ref(&*value)))]
#[ensures(**value == 8)] // expected-error: postcondition might not hold
fn visit(value: &mut &mut u32) {
    let output = identity(value);
    prusti_assert!(**output == 1);
    **output = 7;
    prusti_assert!(**output == 7);
}
#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut &mut value);
    assert_eq!(value, 7);
}
