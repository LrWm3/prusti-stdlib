use iterator_specs::advance;
use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(values.len() > 0)]
fn wrong<T>(values: &[T]) {
    let mut iterator = values.iter();
    let before = ghost! { iterator.remaining() };
    let item = advance(&mut iterator);
    if let Some(_) = item {
        prusti_assert!(iterator.remaining() == *before); // expected-error: assertion might not hold
    }
}

fn main() {}
