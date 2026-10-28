use prusti_contracts::*;
use target_api::ConditionalContainer;

fn main() {
    let result = ConditionalContainer::<usize>::conditional(0);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}
