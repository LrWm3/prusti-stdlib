use prusti_contracts::*;
use target_api::ConditionalContainer;

fn generic<T>() {
    let container = ConditionalContainer::<T>::new();
    prusti_assert!(container.value == 0); // expected-error: assertion might not hold
}

fn main() {}
