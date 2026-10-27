use prusti_contracts::*;

#[requires(mid <= values.len())]
#[requires(first < mid)]
#[requires(second < values.len() - mid)]
#[ensures(Seq::from_slice(values) == old(Seq::from_slice(&*values)))]
fn separate<Element>(values: &mut [Element], mid: usize, first: usize, second: usize) {
    let before = ghost! { Seq::from_slice(&*values) };
    let (left, right) = values.split_at_mut(mid);
    left.swap(first, first);
    right.swap(second, second);
    prusti_assert!(Seq::from_slice(&*left) == (*before)[..mid]);
    prusti_assert!(Seq::from_slice(&*right) == (*before)[mid..]);
    prusti_assert!(Seq::from_slice(values) == *before);
}

fn main() {
    let mut values = [1usize, 2, 3, 4];
    separate(&mut values, 2, 1, 1);
    assert!(values[0] == 1);
    assert!(values[3] == 4);
}
