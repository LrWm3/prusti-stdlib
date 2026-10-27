use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(expected == values.len())]
fn wrong<T>(values: Vec<T>, expected: usize) {
    let mut count = 0usize;
    prusti_for!(iterator, for _value in values => {
        body_invariant!(Int::from(count) + iterator.remaining().len() == Int::from(expected)); // expected-error: loop invariant might not hold on entry
        count += 1;
    });
}

fn main() {}
