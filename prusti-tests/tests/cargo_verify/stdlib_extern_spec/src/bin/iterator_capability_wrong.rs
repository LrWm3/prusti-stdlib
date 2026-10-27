use prusti_contracts::*;
use stdlib_specs::ModeledIterator;

fn modeled<I: ModeledIterator>(iterator: &mut I) {
    let before = ghost! { iterator.remaining() };
    let item = iterator.next();
    if let None = item {
        prusti_assert!((*before).len() > Int::from(0)); // expected-error: assertion might not hold
    }
}

fn main() {}
