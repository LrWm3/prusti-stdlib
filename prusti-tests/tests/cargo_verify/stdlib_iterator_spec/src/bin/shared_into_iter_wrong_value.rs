use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(values.len() > 0)]
fn wrong<T>(values: &[T]) {
    let iterator = ::core::iter::IntoIterator::into_iter(values);
    prusti_assert!(*iterator.remaining()[0] !== &values[0]); // expected-error: assertion might not hold
}

fn main() {}
