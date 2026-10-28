use prusti_contracts::*;
use std::collections::HashSet;

fn main() {
    let mut set = HashSet::<u32>::new();
    set.insert(2);
    set.insert(3);
    set.remove(&2);
    let len = set.len();
    prusti_assert!(len == 2); // expected-error: assertion might not hold
}
