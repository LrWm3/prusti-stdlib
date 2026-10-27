use prusti_contracts::{iter_model::ModeledIterator, *};

fn wrong() {
    prusti_assert!(for_iterator::<std::vec::IntoIter<u8>>().remaining().len() >= Int::from(0)); // expected-unsupported: cannot encode method body `wrong`: for_iterator requires a unique matching iterator in the enclosing for loop
}

fn main() {}
