use prusti_contracts::*;

#[requires(values.len() >= 2 && values[1] == 3)]
fn visit(values: &mut [u32]) {
    let count = iterator_specs::set_prefix_native(values, 1);
    prusti_assert!(count == 1);
    prusti_assert!(values[1] == 7); // expected-error: assertion might not hold
}

fn main() {}
