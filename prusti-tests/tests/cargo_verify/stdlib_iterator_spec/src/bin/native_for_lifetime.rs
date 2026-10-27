use prusti_contracts::{iter_model::ModeledIterator, *};

fn wrong<'a>(values: &'a [u8]) {
    for _value in values {
        body_invariant!(for_iterator::<std::slice::Iter<'static, u8>>().remaining().len() >= Int::from(0)); // expected-unsupported: cannot encode method body `wrong`: for_iterator does not yet support iterator types containing lifetimes
    }
}

fn main() {}
