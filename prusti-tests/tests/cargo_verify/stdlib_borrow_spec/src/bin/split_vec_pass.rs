use borrow_specs::view;
use prusti_contracts::*;

#[requires(mid >= 2)]
#[requires(mid <= values.len())]
#[requires(values.len() - mid >= 2)]
#[ensures(view(values) == old(view(&*values))[..mid]
    .update(0, &*old(view(&*values))[1])
    .update(1, &*old(view(&*values))[0])
    .append(old(view(&*values))[mid..]
        .update(0, &*old(view(&*values))[Int::from(mid) + Int::from(1)])
        .update(1, &*old(view(&*values))[mid])))]
fn exchange<Element>(values: &mut Vec<Element>, mid: usize) {
    let left_before = ghost! { view(&*values)[..mid] };
    let right_before = ghost! { view(&*values)[mid..] };
    let slice = values.as_mut_slice();
    let (left, right) = slice.split_at_mut(mid);
    left.swap(0, 1);
    right.swap(0, 1);
    prusti_assert!(
        Seq::from_slice(&*left)
            == (*left_before)
                .update(0, &*left_before[1])
                .update(1, &*left_before[0])
            && Seq::from_slice(&*right)
                == (*right_before)
                    .update(0, &*right_before[1])
                    .update(1, &*right_before[0])
    );
    prusti_assert!(
        view(values)
            == (*left_before)
                .update(0, &*left_before[1])
                .update(1, &*left_before[0])
                .append(
                    (*right_before)
                        .update(0, &*right_before[1])
                        .update(1, &*right_before[0])
                )
    );
}

fn main() {
    let mut values = Vec::new();
    values.push(1usize);
    values.push(2);
    values.push(3);
    values.push(4);
    exchange(&mut values, 2);
    assert!(values[0] == 2);
    assert!(values[1] == 1);
    assert!(values[2] == 4);
    assert!(values[3] == 3);
}
