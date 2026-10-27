use prusti_contracts::*;

#[requires(values.len() > 0)]
fn wrong<T>(values: Vec<T>) {
    let mut iterator = values.into_iter();
    let item = iterator.next();
    prusti_assert!(matches!(item, None)); // expected-error: assertion might not hold
}

fn main() {}
