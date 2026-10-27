use prusti_contracts::{iter_model::ModeledIterator, *};

fn direct<'a, T>(values: &'a [T]) {
    let mut iterator = values.iter();
    let before = ghost! { iterator.remaining() };
    let item = iterator.next();
    match item {
        Some(value) => {
            prusti_assert!(values.len() > 0);
            prusti_assert!(value === &values[0]);
            prusti_assert!(iterator.remaining() == (*before)[1..]);
        }
        None => {
            prusti_assert!(values.len() == 0);
            let again = iterator.next();
            prusti_assert!(matches!(again, None));
        }
    }
}

fn main() {
    let values = [11usize, 22];
    let slice: &[usize] = &values;
    let mut iterator = slice.iter();
    let first = iterator.next().unwrap();
    prusti_assert!(first === &slice[0]);
    assert!(*first == 11);
    let second = iterator.next().unwrap();
    prusti_assert!(second === &slice[1]);
    assert!(*second == 22);
    assert!(matches!(iterator.next(), None));
    direct(&values);
}
