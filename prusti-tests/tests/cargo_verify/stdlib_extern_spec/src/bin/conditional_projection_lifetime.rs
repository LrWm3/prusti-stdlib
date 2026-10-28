use prusti_contracts::*;
use target_api::ConditionalContainer;

fn generic<'a, 'b>(_left: &'a u8, _right: &'b u8) {
    let result = ConditionalContainer::<&'a u8>::projected::<&'b u8>();
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
