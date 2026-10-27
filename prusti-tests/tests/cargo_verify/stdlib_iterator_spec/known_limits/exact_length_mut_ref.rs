use prusti_contracts::{iter_model::ModeledIterator, *};

fn observe<I: ModeledIterator + ExactSizeIterator>(iterator: &mut I) {
    let length = iterator.len();
    let model = ghost! { iterator.remaining() };
    // The same relationship verifies when the parameter is `&I`.
    prusti_assert!(Int::from(length) == (*model).len());
}

fn main() {}
