use prusti_contracts::*;

fn visit(values: &mut [u32], stop: usize) {
    iterator_specs::set_prefix_native(values, stop);
    prusti_assert!(false); // expected-error: assertion might not hold
}

fn main() {}
