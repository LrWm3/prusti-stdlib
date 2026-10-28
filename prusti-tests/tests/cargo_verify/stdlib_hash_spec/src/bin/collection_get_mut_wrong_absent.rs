use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::<u32, u32>::new();
    map.insert(2, 8);
    let absent = map.get_mut(&3);
    prusti_assert!(matches!(absent, Some(_))); // expected-error: assertion might not hold
}
