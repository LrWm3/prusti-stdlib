use iterator_specs::count_shared_for_source;
use prusti_contracts::*;

struct Token(usize);

fn generic<T>(values: &[T]) {
    let length = values.len();
    let count = count_shared_for_source(values, length);
    prusti_assert!(count == length);
}

fn main() {
    let values = [Token(11), Token(22)];
    generic(&values);
}
