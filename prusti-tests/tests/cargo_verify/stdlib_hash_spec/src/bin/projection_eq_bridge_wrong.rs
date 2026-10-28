use key_specs::{equivalent, Projection};
use prusti_contracts::*;

#[requires(equivalent(left, right))]
fn wrong<K: Projection>(left: &K, right: &K) {
    let equal = left == right;
    prusti_assert!(equal); // expected-error: assertion might not hold
}
fn main() {}
