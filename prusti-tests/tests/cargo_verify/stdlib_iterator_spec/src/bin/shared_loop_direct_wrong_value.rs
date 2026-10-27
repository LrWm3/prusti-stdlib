use iterator_specs::advance;
use prusti_contracts::{iter_model::ModeledIterator, *};

fn wrong<T>(values: &[T]) {
    let mut iterator = values.iter();
    let before = ghost! { iterator.remaining() };
    let mut seen = 0usize;
    loop {
        body_invariant!(seen <= values.len());
        body_invariant!(iterator.remaining() == (*before)[seen..]);
        body_invariant!(forall(|i: usize| i < values.len() ==> *(*before)[i] === &values[i]));
        match advance(&mut iterator) {
            Some(value) => {
                prusti_assert!(value !== &values[seen]); // expected-error: assertion might not hold
                seen += 1;
            }
            None => break,
        }
    }
}

fn main() {}
