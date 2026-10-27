use collection_specs::view;
use prusti_contracts::*;

#[requires(start <= end)]
#[requires(end <= values.len())]
fn ranges<T>(values: &Vec<T>, start: usize, end: usize) {
    let part = &values[start..end];
    assert!(part.len() == end - start);
    prusti_assert!(Seq::from_slice(part) == view(values)[start..end]);
    prusti_assert!(Seq::from_slice(&values[..end]) == view(values)[..end]);
    prusti_assert!(Seq::from_slice(&values[start..]) == view(values)[start..]);
    prusti_assert!(Seq::from_slice(&values[..]) == view(values));
    let checked = values.get(start..end).unwrap();
    prusti_assert!(Seq::from_slice(checked) == Seq::from_slice(part));
    let (left, right) = collection_specs::split(values, start);
    assert!(left.len() == start);
    assert!(right.len() == values.len() - start);
    prusti_assert!(Seq::from_slice(left).append(Seq::from_slice(right)) == view(values));
}

fn lookup<T>(values: &[T], index: usize) {
    match values.get(index) {
        Some(value) => {
            assert!(index < values.len());
            prusti_assert!(*value === values[index]);
        }
        None => assert!(index >= values.len()),
    }
}

#[requires(start <= end)]
#[requires(end < values.len())]
fn inclusive<T>(values: &[T], start: usize, end: usize) {
    let part = &values[start..=end];
    assert!(part.len() == end - start + 1);
    prusti_assert!(
        Seq::from_slice(part)
            == Seq::from_slice(values)[Int::from(start)..Int::from(end) + Int::from(1)]
    );
    let prefix = &values[..=end];
    assert!(prefix.len() == end + 1);
}

fn checked_ranges<T>(values: &[T], start: usize, end: usize) {
    match values.get(start..end) {
        Some(part) => {
            assert!(start <= end);
            assert!(end <= values.len());
            prusti_assert!(Seq::from_slice(part) == Seq::from_slice(values)[start..end]);
        }
        None => assert!(start > end || end > values.len()),
    }
    match values.get(start..=end) {
        Some(part) => {
            assert!(end < values.len());
            prusti_assert!(Int::from(start) <= Int::from(end) + Int::from(1));
            prusti_assert!(
                Seq::from_slice(part)
                    == Seq::from_slice(values)[Int::from(start)..Int::from(end) + Int::from(1)]
            );
        }
        None => {
            prusti_assert!(end >= values.len() || Int::from(start) > Int::from(end) + Int::from(1))
        }
    }
}

#[requires(a < values.len())]
#[requires(b < values.len())]
fn swap<T>(values: &mut [T], a: usize, b: usize) {
    let before = ghost! { Seq::from_slice(&*values) };
    values.swap(a, b);
    prusti_assert!(values[a] === *before[b]);
    prusti_assert!(values[b] === *before[a]);
    prusti_assert!(forall(|i: usize| (i < values.len() && i != a && i != b) ==> values[i] === *before[i]));
    values.swap(a, b);
    prusti_assert!(Seq::from_slice(values) == *before);
}

fn main() {
    let values = collection_specs::make(8);
    ranges(&values, 2, 7);
    inclusive(&values, 2, 6);
    lookup(&values, 7);
    lookup(&values, 8);
    checked_ranges(&values, 7, 3);
    checked_ranges(&values, 3, 4);
    checked_ranges(&values, 0, usize::MAX);
    let mut values = [
        collection_specs::Owned { value: 2 },
        collection_specs::Owned { value: 7 },
    ];
    swap(&mut values, 0, 1);
    swap(&mut values, 1, 1);
}
