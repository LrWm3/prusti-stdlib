use collection_specs::view;
use prusti_contracts::*;

#[requires(i < dest.len())]
#[requires(j < src.len())]
#[requires(Int::from(dest.len()) + Int::from(src.len()) <= Int::from(usize::MAX))]
fn append<T>(dest: &mut Vec<T>, src: &mut Vec<T>, i: usize, j: usize) {
    let left = ghost! { view(&*dest) };
    let right = ghost! { view(&*src) };
    let n = dest.len();
    let m = src.len();
    collection_specs::concatenate(dest, src);
    assert!(dest.len() == n + m);
    assert!(src.is_empty());
    prusti_assert!(dest[i] === *left[i]);
    prusti_assert!(dest[n + j] === *right[j]);
}

#[requires(i <= values.len())]
#[requires(values.len() < usize::MAX)]
fn insert<T>(values: &mut Vec<T>, item: T, i: usize) {
    let before = ghost! { view(&*values) };
    let moved = ghost! { Ghost::new_ref(&item) };
    let n = values.len();
    values.insert(i, item);
    assert!(values.len() == n + 1);
    prusti_assert!(values[i] === **moved);
    prusti_assert!(forall(|j: usize| j < i ==> values[j] === *before[j]));
    prusti_assert!(forall(|j: usize| (i <= j && j < n) ==> values[j + 1] === *before[j]));
}

#[requires(i < values.len())]
fn remove<T>(values: &mut Vec<T>, i: usize) {
    let before = ghost! { view(&*values) };
    let n = values.len();
    let item = values.remove(i);
    assert!(values.len() == n - 1);
    prusti_assert!(item === *before[i]);
    prusti_assert!(forall(|j: usize| j < i ==> values[j] === *before[j]));
    prusti_assert!(forall(|j: usize| (i <= j && j < values.len()) ==> values[j] === *before[j + 1]));
}

#[requires(i < values.len())]
fn swap_remove<T>(values: &mut Vec<T>, i: usize) {
    let before = ghost! { view(&*values) };
    let n = values.len();
    let item = values.swap_remove(i);
    assert!(values.len() == n - 1);
    prusti_assert!(item === *before[i]);
    prusti_assert!(i < values.len() ==> values[i] === *before[n - 1]);
    prusti_assert!(forall(|j: usize| (j < values.len() && j != i) ==> values[j] === *before[j]));
}

fn truncate<T>(values: &mut Vec<T>, n: usize) {
    let before = ghost! { view(&*values) };
    let len = values.len();
    values.truncate(n);
    assert!(values.len() == if n < len { n } else { len });
    prusti_assert!(forall(|j: usize| j < values.len() ==> values[j] === *before[j]));
    values.clear();
    assert!(values.is_empty());
}

fn main() {
    let mut left = collection_specs::make(8);
    let mut right = collection_specs::make(5);
    append(&mut left, &mut right, 6, 4);
    let mut values = collection_specs::make(8);
    insert(&mut values, collection_specs::Owned { value: 19 }, 4);
    let mut values = collection_specs::make(8);
    remove(&mut values, 3);
    let mut values = collection_specs::make(8);
    swap_remove(&mut values, 3);
    let mut values = collection_specs::make(8);
    truncate(&mut values, 5);
}
