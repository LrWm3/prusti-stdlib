use iterator_specs::count_shared;
use prusti_contracts::*;

fn generic<T>(values: &[T]) {
    let expected = values.len();
    let count = count_shared(values, expected);
    prusti_assert!(count == expected);
}

fn main() {
    let values = [11usize, 22, 33];
    assert!(count_shared(&values, 3) == 3);
    assert!(count_shared::<usize>(&[], 0) == 0);
    generic(&values);
}
