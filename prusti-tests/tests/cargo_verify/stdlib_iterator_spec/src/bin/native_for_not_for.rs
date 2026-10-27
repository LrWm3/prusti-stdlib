use prusti_contracts::{iter_model::ModeledIterator, *};

fn wrong(values: Vec<u8>) {
    let mut iterator = values.into_iter();
    loop {
        match iterator.next() {
            Some(_) => {
                body_invariant!(for_iterator::<std::vec::IntoIter<u8>>().remaining().len() >= Int::from(0)); // expected-unsupported: cannot encode method body `wrong`: for_iterator requires a unique matching iterator in the enclosing for loop
            }
            None => break,
        }
    }
}

fn main() {}
