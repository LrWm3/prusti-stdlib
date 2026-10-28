use prusti_contracts::*;
use target_api::ConditionalContainer;

fn observe<'a>(_value: &'a u8) {
    let result = ConditionalContainer::<&'a u8>::static_observe();
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
