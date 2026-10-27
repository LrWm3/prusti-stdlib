use prusti_contracts::*;

#[rustfmt::skip]
fn main() {
    prusti_assert!(matches!(prusti_contracts::core_spec::slice::range_index_model(&(0usize..=2)), Some(_))); // expected-error: assertion might not hold
}
