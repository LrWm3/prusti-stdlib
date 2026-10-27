use iterator_specs::advance;
use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(expected == values.len())]
#[ensures(result == expected)]
fn count<T>(values: &[T], expected: usize) -> usize {
    let mut iterator = values.iter();
    let before = ghost! { iterator.remaining() };
    let mut seen = 0usize;
    loop {
        body_invariant!(expected == old(expected));
        body_invariant!(seen <= expected);
        body_invariant!((*before).len() == Int::from(expected));
        body_invariant!(iterator.remaining() == (*before)[seen..]);
        body_invariant!(forall(|i: usize| i < expected ==> *(*before)[i] === &values[i]));
        match advance(&mut iterator) {
            Some(value) => {
                prusti_assert!(value === &values[seen]);
                seen += 1;
            }
            None => {
                prusti_assert!(seen == expected);
                break;
            }
        }
    }
    seen
}

fn main() {
    let values = [10, 20];
    assert_eq!(count(&values, 2), 2);
}
