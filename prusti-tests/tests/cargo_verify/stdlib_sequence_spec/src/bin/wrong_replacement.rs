use prusti_contracts::*;
use sequence_specs::vec_contents as view;

#[requires(sequence_specs::vec_contents(&*dest) != sequence_specs::vec_contents(&src))]
fn wrong<T>(dest: &mut Vec<T>, src: Vec<T>) {
    let previous = sequence_specs::replace_vec(dest, src);
    prusti_assert!(view(&previous) == view(dest)); // expected-error: assertion might not hold
}

fn main() {}
