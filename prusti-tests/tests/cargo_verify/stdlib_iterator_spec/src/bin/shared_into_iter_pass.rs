use iterator_specs::count_shared_for;
use prusti_contracts::{iter_model::ModeledIterator, *};

fn constructor<'a, T>(values: &'a [T]) {
    let iterator = ::core::iter::IntoIterator::into_iter(values);
    prusti_assert!(iterator.remaining().len() == Int::from(values.len()));
    prusti_assert!(forall(|i: usize| i < values.len() ==> *iterator.remaining()[i] === &values[i]));
}

fn generic_count<T>(values: &[T]) {
    let length = values.len();
    let count = count_shared_for(values, length);
    prusti_assert!(count == length);
}

fn main() {
    let values = [11usize, 22];
    constructor(&values);
    generic_count(&values);
}
