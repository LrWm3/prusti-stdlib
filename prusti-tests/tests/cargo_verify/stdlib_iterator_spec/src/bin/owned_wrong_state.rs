use iterator_specs::view;
use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(values.len() > 0)]
fn wrong<T>(values: Vec<T>) {
    let before = ghost! { view(&values) };
    let mut iterator = values.into_iter();
    let item = iterator.next();
    if let Some(_) = item {
        prusti_assert!(iterator.remaining() == *before); // expected-error: assertion might not hold
    }
}

fn main() {}
