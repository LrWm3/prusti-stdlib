use prusti_contracts::{iter_model::ModeledIterator, *};

fn count<T>(values: Vec<T>) {
    let mut iterator = values.into_iter();
    for _ in iterator.by_ref() {
        // The `for` loop holds a mutable borrow for its entire duration, so
        // Rust rejects the invariant's attempt to read the named iterator.
        body_invariant!(iterator.remaining().len() >= Int::from(0));
    }
}

fn main() {}
