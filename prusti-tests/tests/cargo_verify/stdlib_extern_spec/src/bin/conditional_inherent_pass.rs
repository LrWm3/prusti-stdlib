use prusti_contracts::*;
use stdlib_specs::ConditionalModel;
use target_api::ConditionalContainer;

fn generic<T: ConditionalModel>() {
    let mut container = ConditionalContainer::<T>::new();
    prusti_assert!(container.value == 0);
    let previous = container.replace(9);
    prusti_assert!(previous == 0);
    prusti_assert!(container.value == 9);
    let previous = container.replace(4);
    prusti_assert!(previous == 9);
}

fn main() {
    generic::<usize>();
}
