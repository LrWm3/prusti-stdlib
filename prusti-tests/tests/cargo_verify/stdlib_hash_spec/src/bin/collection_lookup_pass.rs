use key_specs::Key;
use prusti_contracts::hash_model::HashKeyModel;
use prusti_contracts::*;
use std::collections::{HashMap, HashSet};

fn generic<K: HashKeyModel>(map: &HashMap<K, u32>, key: &K) {
    let present = map.contains_key(key);
    prusti_assert!(present == prusti_contracts::std_spec::hash::map_contents(map).contains(key.key_model()));
    match map.get(key) {
        Some(value) => {
            prusti_assert!(present);
            prusti_assert!(*value === *prusti_contracts::std_spec::hash::map_contents(map)[key.key_model()]);
        }
        None => { prusti_assert!(!present); }
    }
}

fn main() {
    let mut map = HashMap::<Key, u32>::new();
    map.insert(Key { id: 2, ignored: 1 }, 8);
    map.insert(Key { id: 3, ignored: 1 }, 9);
    let equal = Key { id: 2, ignored: 9 };
    let other = Key { id: 3, ignored: 0 };
    let missing = Key { id: 4, ignored: 1 };
    generic(&map, &equal);
    let present = map.contains_key(&equal);
    let absent = map.contains_key(&missing);
    prusti_assert!(present);
    prusti_assert!(!absent);
    match map.get(&equal) {
        Some(value) => { prusti_assert!(*value == 8); }
        None => { prusti_assert!(false); }
    }
    match map.get(&other) {
        Some(value) => { prusti_assert!(*value == 9); }
        None => { prusti_assert!(false); }
    }
    let absent_value = map.get(&missing);
    prusti_assert!(matches!(absent_value, None));
    let mut set = HashSet::<Key>::new();
    set.insert(Key { id: 2, ignored: 1 });
    set.insert(Key { id: 3, ignored: 1 });
    let present = set.contains(&equal);
    let absent = set.contains(&missing);
    prusti_assert!(present);
    prusti_assert!(!absent);
}
