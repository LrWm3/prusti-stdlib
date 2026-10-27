use prusti_contracts::{iter_model::ModeledIterator, *};

fn shared<'a, T>(iterator: &mut core::slice::Iter<'a, T>) {
    let before = ghost! { iterator.remaining() };
    let item = iterator.next();
    if let None = item {
        prusti_assert!((*before).len() == Int::from(0));
    }
}

fn main() {}
