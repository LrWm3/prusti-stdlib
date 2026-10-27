use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(expected == values.len())]
fn wrong<T>(values: &[T], expected: usize) {
    let mut count = 0usize;
    prusti_for!(iterator, for value in values => init {
        let before = ghost! { iterator.remaining() };
    } {
        body_invariant!(expected == old(expected));
        body_invariant!(count < expected);
        body_invariant!((*before).len() == Int::from(expected));
        body_invariant!(iterator.remaining() == (*before)[count + 1..]);
        body_invariant!(forall(|i: usize| i < expected ==> *(*before)[i] === &values[i]));
        body_invariant!(value === &values[count]);
        prusti_assert!(value !== &values[count]); // expected-error: assertion might not hold
        count += 1;
    });
}

fn main() {}
