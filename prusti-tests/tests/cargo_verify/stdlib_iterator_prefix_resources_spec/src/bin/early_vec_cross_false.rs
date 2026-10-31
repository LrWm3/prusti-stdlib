use prusti_contracts::*;
#[requires(!values.is_empty())]
#[requires(limit <= values.len())]
fn check(values: &mut Vec<&mut u32>, limit: usize) {
    iterator_prefix_resource_specs::set_prefix(values.as_mut_slice(), limit);
    prusti_assert!(false); // expected-error: assertion might not hold
}
#[trusted]
fn main() {
    let mut a = 1;
    check(&mut vec![&mut a], 1);
}
