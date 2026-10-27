use iterator_specs::advance;
use prusti_contracts::{iter_model::ModeledIterator, *};

fn borrowed<'a, T>(iterator: &mut std::vec::IntoIter<&'a T>) {
    let before = ghost! { iterator.remaining() };
    let item = advance(iterator);
    if let None = item {
        prusti_assert!((*before).len() > Int::from(0)); // expected-error: assertion might not hold
    }
}

fn main() {}
