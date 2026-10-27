use prusti_contracts::{iter_model::ModeledIterator, *};

fn borrowed<'a, T>(iterator: &mut std::vec::IntoIter<&'a T>) {
    let before = ghost! { iterator.remaining() };
    let item = iterator.next();
    match item {
        Some(value) => {
            prusti_assert!(value === *(*before)[0]);
            prusti_assert!(iterator.remaining() == (*before)[1..]);
        }
        None => {
            prusti_assert!((*before).len() == Int::from(0));
            prusti_assert!(iterator.remaining() == *before);
        }
    }
}

fn main() {}
