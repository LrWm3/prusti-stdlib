use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(expected == values.len())]
#[ensures(result == expected)]
fn count<T>(values: Vec<T>, expected: usize) -> usize {
    let mut other = Vec::<T>::new().into_iter();
    let mut count = 0usize;
    for _value in values {
        let _ = other.next();
        body_invariant!(expected == old(expected));
        body_invariant!(Int::from(count)
            + for_iterator::<std::vec::IntoIter<T>>().remaining().len()
            + Int::from(1) == Int::from(expected));
        count += 1;
    }
    count
}

fn main() {}
