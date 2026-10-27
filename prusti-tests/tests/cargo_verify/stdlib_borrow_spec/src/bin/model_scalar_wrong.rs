use prusti_contracts::*;

#[rustfmt::skip]
fn main() {
    prusti_assert!(matches!(prusti_contracts::core_spec::slice::range_index_model(&1usize), Some(_))); // expected-error: assertion might not hold
}
