use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::<u32, u32>::new();
    map.insert(2, 8);
    map.insert(2, 9);
    let len = map.len();
    prusti_assert!(len == 2); // expected-error: assertion might not hold
}
