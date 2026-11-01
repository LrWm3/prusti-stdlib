use prusti_contracts::{iter_model::ModeledIterator, *};
#[requires(expected == values.len())]
#[ensures(result <= expected)]
#[ensures(!stop || result == 0)]
fn count<T>(values: Vec<T>, expected: usize, stop: bool) -> usize {
    let mut count = 0usize;
    for _value in values {
        if stop {

            return count;
        }
        body_invariant!(expected == old(expected));
        body_invariant!(stop == old(stop));
        body_invariant!(Int::from(count)
            + for_iterator::<std::vec::IntoIter<T>>().remaining().len()
            + Int::from(1) == Int::from(expected));
        count += 1;
    }
    count
}
struct Token(u32);
#[trusted]
fn main() {
    assert_eq!(count(Vec::<Token>::new(), 0, true), 0);
    assert_eq!(count(vec![Token(1), Token(2)], 2, true), 0);
    assert_eq!(count(vec![Token(1), Token(2)], 2, false), 2);
}
