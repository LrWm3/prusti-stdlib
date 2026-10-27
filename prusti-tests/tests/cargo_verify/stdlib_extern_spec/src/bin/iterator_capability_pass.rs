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

fn modeled_trait_call<I: ModeledIterator>(iterator: &mut I) {
    let before = ghost! { iterator.remaining() };
    let item = iterator.next();
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
