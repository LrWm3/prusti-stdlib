use iterator_specs::{count_owned, view};
use prusti_contracts::{iter_model::ModeledIterator, *};

struct Token {
    value: usize,
}

fn first<T>(values: Vec<T>) {
    let before = ghost! { view(&values) };
    let mut iterator = values.into_iter();
    prusti_assert!(iterator.remaining() == *before);
    match iterator.next() {
        Some(value) => {
            prusti_assert!(value === *(*before)[0]);
            prusti_assert!(iterator.remaining() == (*before)[1..]);
        }
        None => {
            prusti_assert!((*before).len() == Int::from(0));
            prusti_assert!(iterator.remaining() == *before);
            let again = iterator.next();
            prusti_assert!(matches!(again, None));
        }
    }
}

fn generic_count<T>(values: Vec<T>) {
    let len = values.len();
    let count = count_owned(values, len);
    prusti_assert!(count == len);
}

fn main() {
    let mut values = Vec::new();
    values.push(Token { value: 11 });
    values.push(Token { value: 22 });
    let mut iterator = values.into_iter();
    assert!(iterator.next().unwrap().value == 11);
    assert!(iterator.next().unwrap().value == 22);
    assert!(matches!(iterator.next(), None));
    assert!(matches!(iterator.next(), None));
    first(Vec::<Token>::new());
    generic_count(Vec::<Token>::new());
}
