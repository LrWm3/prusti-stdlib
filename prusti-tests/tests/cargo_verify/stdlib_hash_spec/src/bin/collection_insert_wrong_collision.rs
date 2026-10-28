use key_specs::Key;
use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::<Key, u32>::new();
    map.insert(Key { id: 2, ignored: 1 }, 8);
    let previous = map.insert(Key { id: 3, ignored: 1 }, 9);
    prusti_assert!(matches!(previous, Some(_))); // expected-error: assertion might not hold
}
