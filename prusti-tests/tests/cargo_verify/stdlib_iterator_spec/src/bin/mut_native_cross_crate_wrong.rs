use prusti_contracts::*;

#[requires(values.len() == 2 && values[0] == 1 && values[1] == 2)]
fn wrong(values: &mut [u32]) {
    iterator_specs::set_first_native(values);
    prusti_assert!(values[1] == 7); // expected-error: assertion might not hold
}

fn main() {}
