use prusti_contracts::*;

fn wrong<K: Eq>(left: &K, right: &K) {
    let equal = left == right;
    prusti_assert!(equal); // expected-error: assertion might not hold
}
fn main() {}
