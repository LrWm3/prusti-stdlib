use prusti_contracts::*;
use stdlib_specs::SlotModel;
use target_api::ConditionalContainer;

fn generic<T: SlotModel>() {
    let result = ConditionalContainer::<T>::projected::<T>();
    prusti_assert!(result == 7);
}

fn main() {
    generic::<usize>();
}
