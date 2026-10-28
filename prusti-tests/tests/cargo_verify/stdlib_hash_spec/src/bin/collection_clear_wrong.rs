use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::<u32, u32>::new();
    map.insert(2, 8);
    map.clear();
    let empty = map.is_empty();
    prusti_assert!(!empty); // expected-error: assertion might not hold
}
