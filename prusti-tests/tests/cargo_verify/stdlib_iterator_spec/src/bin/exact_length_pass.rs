use prusti_contracts::*;
use iterator_specs::{advance_with_lengths, remaining_len};

fn owned<T>(values: Vec<T>) {
    let expected = values.len();
    let mut iterator = values.into_iter();
    let before = iterator.len();
    prusti_assert!(before == expected);
    let modeled = remaining_len(&iterator);
    prusti_assert!(modeled == before);
    match iterator.next() {
        Some(_) => {
            let after = iterator.len();
            prusti_assert!(after + 1 == expected);
        }
        None => {
            let after = iterator.len();
            prusti_assert!(expected == 0);
            prusti_assert!(after == 0);
        }
    }
    let (before, after) = advance_with_lengths(&mut iterator);
    prusti_assert!(after <= before);
}

fn shared<T>(values: &[T]) {
    let expected = values.len();
    let mut iterator = values.iter();
    let before = iterator.len();
    prusti_assert!(before == expected);
    let modeled = remaining_len(&iterator);
    prusti_assert!(modeled == before);
    match iterator.next() {
        Some(_) => {
            let after = iterator.len();
            prusti_assert!(after + 1 == expected);
        }
        None => {
            let after = iterator.len();
            prusti_assert!(expected == 0);
            prusti_assert!(after == 0);
        }
    }
    let (before, after) = advance_with_lengths(&mut iterator);
    prusti_assert!(after <= before);
}

struct Token(u32);
fn main() {
    let mut values = Vec::new();
    values.push(Token(1));
    values.push(Token(2));
    shared(values.as_slice());
    owned(values);
    owned(Vec::<Token>::new());
    shared(Vec::<Token>::new().as_slice());
}
