use iterator_specs::{advance, first_shared};
use prusti_contracts::{iter_model::ModeledIterator, *};

struct Token {
    value: usize,
}

fn first<'a, T>(values: &'a [T]) {
    let mut iterator = values.iter();
    let result = advance(&mut iterator);
    match result {
        Some(value) => {
            prusti_assert!(values.len() > 0);
            prusti_assert!(value === &values[0]);
            prusti_assert!(iterator.remaining().len() == Int::from(values.len()) - Int::from(1));
        }
        None => {
            prusti_assert!(values.len() == 0);
            let again = advance(&mut iterator);
            prusti_assert!(matches!(again, None));
        }
    }
}

#[requires(values.len() > 0)]
fn cross_crate<'a, T>(values: &'a [T]) {
    let first = first_shared(values);
    prusti_assert!(first === &values[0]);
}

fn main() {
    let values = [Token { value: 11 }, Token { value: 22 }];
    let slice: &[Token] = &values;
    let mut iterator = slice.iter();
    let first_item = advance(&mut iterator).unwrap();
    prusti_assert!(first_item === &slice[0]);
    prusti_assert!(first_item.value == slice[0].value);
    assert!(first_item.value == 11);
    let second_item = advance(&mut iterator).unwrap();
    prusti_assert!(second_item === &slice[1]);
    prusti_assert!(second_item.value == slice[1].value);
    assert!(second_item.value == 22);
    assert!(matches!(advance(&mut iterator), None));
    assert!(matches!(advance(&mut iterator), None));
    assert!(first_shared(&values).value == 11);
    first::<Token>(&[]);
    cross_crate(&values);
}
