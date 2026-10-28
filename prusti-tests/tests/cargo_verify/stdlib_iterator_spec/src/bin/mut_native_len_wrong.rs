use prusti_contracts::*;

#[requires(values.len() == 2)]
fn wrong<T>(values: &mut [T]) {
    let it = values.iter_mut();
    prusti_assert!(it.len() == 0); // expected-error: assertion might not hold
}

fn main() {}
