use prusti_contracts::*;
use vec_specs::view;

#[requires(i < values.len())]
#[requires(values.len() < usize::MAX)]
fn wrong(values: &mut Vec<usize>, i: usize) {
    let before = ghost! { view(&*values) };
    vec_specs::append_one(values, 7);
    prusti_assert!(values[i] != *before[i]); // expected-error: assertion might not hold
}

fn main() {}
