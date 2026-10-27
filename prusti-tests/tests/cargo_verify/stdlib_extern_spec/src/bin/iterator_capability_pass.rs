use prusti_contracts::*;
use stdlib_specs::{modeled_advance, ModeledIterator};

fn modeled<I: ModeledIterator>(iterator: &mut I) {
    let before = ghost! { iterator.remaining() };
    let item = modeled_advance(iterator);
    match item {
        Some(_) => {
            prusti_assert!((*before).len() > Int::from(0));
        }
        None => {
            prusti_assert!((*before).len() == Int::from(0));
        }
    }
}

fn main() {}
