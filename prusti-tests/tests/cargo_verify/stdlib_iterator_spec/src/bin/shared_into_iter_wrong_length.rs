use prusti_contracts::{iter_model::ModeledIterator, *};

fn wrong<T>(values: &[T]) {
    let iterator = ::core::iter::IntoIterator::into_iter(values);
    prusti_assert!(iterator.remaining().len() != Int::from(values.len())); // expected-error: assertion might not hold
}

fn main() {}
