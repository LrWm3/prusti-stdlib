use prusti_contracts::*;
use stdlib_specs::SlotModel;
use target_api::ConditionalContainer;

fn generic<T: SlotModel, U: SlotModel>() {
    let result = ConditionalContainer::<T>::projected::<U>();
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
