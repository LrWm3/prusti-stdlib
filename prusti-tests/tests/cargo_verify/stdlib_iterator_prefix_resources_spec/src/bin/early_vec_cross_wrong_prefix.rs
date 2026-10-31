use prusti_contracts::*;
#[requires(limit <= values.len())]
#[requires(index < values.len())]
#[requires(index < limit)]
fn check(values: &mut Vec<&mut u32>, limit: usize, index: usize) {
    iterator_prefix_resource_specs::set_prefix(values.as_mut_slice(), limit);
    let actual = *values[index];
    prusti_assert!(actual != 7); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    let (mut a, mut b) = (1, 2);
    check(&mut vec![&mut a, &mut b], 1, 0);
}
