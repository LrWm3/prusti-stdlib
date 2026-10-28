use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    iterator_specs::set_all_native(values);
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {}
