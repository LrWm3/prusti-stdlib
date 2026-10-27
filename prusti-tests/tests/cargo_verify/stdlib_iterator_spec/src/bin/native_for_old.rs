use prusti_contracts::{iter_model::ModeledIterator, *};

fn wrong(values: Vec<u8>) {
    for _value in values {
        body_invariant!(old(for_iterator::<std::vec::IntoIter<u8>>()).remaining().len() >= Int::from(0)); // expected-unsupported: cannot encode method body `wrong`: for_iterator cannot be used in nested or old-state specifications
    }
}

fn main() {}
