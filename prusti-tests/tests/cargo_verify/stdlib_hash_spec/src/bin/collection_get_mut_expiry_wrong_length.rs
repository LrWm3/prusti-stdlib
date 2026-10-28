use prusti_contracts::*;
use std::collections::HashMap;
fn main() {
    let mut map = HashMap::<u32, u32>::new();
    map.insert(2, 8);
    map.insert(3, 9);
    if let Some(value) = map.get_mut(&2) {
        *value = 10;
    }
    let len = map.len();
    prusti_assert!(len == 1); // expected-error: assertion might not hold
}
