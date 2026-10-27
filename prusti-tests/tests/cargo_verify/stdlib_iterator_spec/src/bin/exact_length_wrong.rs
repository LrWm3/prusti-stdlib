use prusti_contracts::*;

#[requires(values.len() > 0)]
fn wrong(values: &[u8]) {
    let iterator = values.iter();
    let length = iterator.len();
    prusti_assert!(length == 0); // expected-error: assertion might not hold
}
fn main() {}
