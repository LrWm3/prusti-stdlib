use prusti_contracts::*;

#[requires(mid <= values.len())]
#[requires(first < mid)]
#[requires(second < values.len() - mid)]
#[ensures(Seq::from_slice(values) == old(Seq::from_slice(&*values))[..mid]
    .update(0, &*old(Seq::from_slice(&*values))[first])
    .update(first, &*old(Seq::from_slice(&*values))[0])
    .append(old(Seq::from_slice(&*values))[mid..]
        .update(0, &*old(Seq::from_slice(&*values))[Int::from(mid) + Int::from(second)])
        .update(second, &*old(Seq::from_slice(&*values))[mid])))]
fn exchange_halves<Element>(values: &mut [Element], mid: usize, first: usize, second: usize) {
    let left_before = ghost! { Seq::from_slice(&*values)[..mid] };
    let right_before = ghost! { Seq::from_slice(&*values)[mid..] };
    let (left, right) = values.split_at_mut(mid);
    left.swap(0, first);
    right.swap(0, second);
    prusti_assert!(
        Seq::from_slice(&*left)
            == (*left_before)
                .update(0, &*left_before[first])
                .update(first, &*left_before[0])
            && Seq::from_slice(&*right)
                == (*right_before)
                    .update(0, &*right_before[second])
                    .update(second, &*right_before[0])
    );
    prusti_assert!(
        Seq::from_slice(values)
            == (*left_before)
                .update(0, &*left_before[first])
                .update(first, &*left_before[0])
                .append(
                    (*right_before)
                        .update(0, &*right_before[second])
                        .update(second, &*right_before[0])
                )
    );
}

fn main() {
    let mut values = [1usize, 2, 3, 4];
    exchange_halves(&mut values, 2, 1, 1);
    assert!(values[0] == 2);
    assert!(values[1] == 1);
    assert!(values[2] == 4);
    assert!(values[3] == 3);
}
