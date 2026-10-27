use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(expected == values.len())]
fn wrong<T>(values: Vec<T>, expected: usize) {
    let mut count = 0usize;
    for _value in values {
        body_invariant!(Int::from(count) + for_iterator::<std::vec::IntoIter<T>>().remaining().len() == Int::from(expected)); // expected-error: loop invariant might not hold on entry
        count += 1;
    }
}

fn main() {}
