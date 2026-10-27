use iterator_specs::count_native_for;
use prusti_contracts::*;

struct Token(usize);

fn generic_count<T>(values: Vec<T>) {
    let length = values.len();
    let count = count_native_for(values, length);
    prusti_assert!(count == length);
}

fn main() {
    generic_count(Vec::<Token>::new());
}
