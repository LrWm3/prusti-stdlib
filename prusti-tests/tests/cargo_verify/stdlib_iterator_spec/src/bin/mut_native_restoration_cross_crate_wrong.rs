use prusti_contracts::*;

#[requires(!values.is_empty())]
fn visit(values: &mut [u32]) {
    iterator_specs::set_all_native(values);
    prusti_assert!(values[0] == 8); // expected-error: assertion might not hold
}

fn main() {}
