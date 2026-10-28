use prusti_contracts::*;
use target_api::ConditionalContainer;

fn main() {
    let result = ConditionalContainer::<usize>::projected::<u8>();
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}
