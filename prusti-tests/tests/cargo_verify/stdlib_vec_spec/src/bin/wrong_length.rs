use prusti_contracts::*;

#[requires(values.len() < usize::MAX)]
fn wrong<T>(values: &mut Vec<T>, item: T) {
    let n = values.len();
    vec_specs::append_one(values, item);
    prusti_assert!(values.len() == n); // expected-error: assertion might not hold
}

fn main() {}
