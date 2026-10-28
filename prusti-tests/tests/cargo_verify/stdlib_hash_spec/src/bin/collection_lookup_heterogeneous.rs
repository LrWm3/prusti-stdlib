use key_specs::Key;
use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let map = HashMap::<Key, u32>::new();
    let present = map.contains_key(&2u32);
    prusti_assert!(!present); // expected-error: assertion might not hold
}
