use prusti_contracts::*;
use std::collections::HashSet;

fn main() {
    let mut set = HashSet::<u32>::new();
    set.insert(2);
    let present = set.contains(&3);
    prusti_assert!(present); // expected-error: assertion might not hold
}
