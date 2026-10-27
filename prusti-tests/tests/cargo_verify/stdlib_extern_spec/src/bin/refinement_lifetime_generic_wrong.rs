use prusti_contracts::*;
use stdlib_specs::ReadRef;
fn wrong<'a, T, R: ReadRef<'a, Item = T>>(r: &R, value: &'a T) {
    let result = r.get();
    prusti_assert!(result === value); // expected-error: assertion might not hold
}
fn main() {}
