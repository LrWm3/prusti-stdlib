use key_specs::Key;
use prusti_contracts::*;
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::<Key, u32>::new();
    map.insert(Key { id: 2, ignored: 1 }, 8);
    map.insert(Key { id: 3, ignored: 1 }, 9);
    match map.get_mut(&Key { id: 2, ignored: 9 }) {
        Some(value) => {
            prusti_assert!(*value == 8);
            *value = 10;
        }
        None => {
            prusti_assert!(false);
        }
    }

    let mut other = HashMap::<u32, u32>::new();
    other.insert(2, 8);
    let absent = other.get_mut(&3);
    prusti_assert!(matches!(absent, None));
}
