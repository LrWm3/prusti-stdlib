use prusti_contracts::*;
#[requires(!values.is_empty())]
#[requires(limit <= values.len())]
fn check(values: &mut [&mut u32], limit: usize) {
    iterator_prefix_resource_specs::set_prefix(values, limit);
    prusti_assert!(false); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    let mut a = 1;
    check(&mut [&mut a], 1);
}
