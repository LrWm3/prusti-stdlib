use iterator_specs::mut_remaining;
use prusti_contracts::*;

#[requires(values.len() == 1)]
fn wrong(values: &mut [u32]) {
    let mut iterator = values.iter_mut();
    prusti_assert!(mut_remaining(&iterator).len() == Int::from(1));
    let next = iterator.next();
    prusti_assert!(matches!(next, None)); // expected-error: assertion might not hold
}
fn main() {}
