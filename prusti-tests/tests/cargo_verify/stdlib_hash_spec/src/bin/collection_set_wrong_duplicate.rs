use prusti_contracts::*;
use std::collections::HashSet;

fn main() {
    let mut set = HashSet::<u32>::new();
    set.insert(2);
    let duplicate = set.insert(2);
    prusti_assert!(duplicate); // expected-error: assertion might not hold
}
