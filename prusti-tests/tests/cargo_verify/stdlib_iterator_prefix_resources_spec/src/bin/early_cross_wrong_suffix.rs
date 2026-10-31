use prusti_contracts::*;
#[requires(limit <= values.len())]
#[requires(index < values.len())]
#[requires(limit <= index)]
fn check(values: &mut [&mut u32], limit: usize, index: usize) {
    let before = *values[index];
    iterator_prefix_resource_specs::set_prefix(values, limit);
    let actual = *values[index];
    prusti_assert!(actual != before); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    let (mut a, mut b) = (1, 2);
    check(&mut [&mut a, &mut b], 1, 1);
}
