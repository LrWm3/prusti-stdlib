use iterator_specs::mut_remaining;
use prusti_contracts::*;

#[requires(values.len() == 1 && values[0] == 1)]
fn wrong(values: &mut [u32]) {
    let mut iterator = values.iter_mut();
    prusti_assert!(mut_remaining(&iterator).len() == Int::from(1));
    if let Some(value) = iterator.next() {
        prusti_assert!(*value == 9); // expected-error: assertion might not hold
    }
}
fn main() {}
