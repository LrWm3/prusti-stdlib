use prusti_contracts::{iter_model::ModeledIterator, *};

#[ensures(result != values.len())] // expected-error: postcondition might not hold
fn count<T>(values: &[T]) -> usize {
    let mut it = values.iter();
    let mut seen = 0;
    loop {
        body_invariant!(values === old(values));
        body_invariant!(seen <= values.len());
        body_invariant!(Int::from(seen) + it.remaining().len() == Int::from(values.len()));
        match it.next() {
            Some(_) => seen += 1,
            None => break,
        }
    }
    seen
}

fn main() {}
