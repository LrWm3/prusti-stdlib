use key_specs::Key;
use prusti_contracts::{hash_model::HashKeyModel, *};
use std::collections::HashMap;

struct Item {
    number: u32,
}

fn noncopy_values() {
    let mut map = HashMap::<u32, Item>::new();
    map.insert(2, Item { number: 8 });
    map.insert(3, Item { number: 9 });
    match map.get_mut(&2) {
        Some(value) => {
            value.number = 10;
        }
        None => {
            prusti_assert!(false);
        }
    }
    prusti_assert!(prusti_contracts::std_spec::hash::map_contents(&map)[2].number == 10);
    prusti_assert!(prusti_contracts::std_spec::hash::map_contents(&map)[3].number == 9);
}

#[requires(prusti_contracts::std_spec::hash::map_contents(&*map).contains(key.key_model()))]
#[ensures(prusti_contracts::std_spec::hash::map_contents(&*map)
    == old(prusti_contracts::std_spec::hash::map_contents(&*map)).insert(key.key_model(), 10))]
fn set_ten<K: HashKeyModel>(map: &mut HashMap<K, u32>, key: &K) {
    match map.get_mut(key) {
        Some(value) => {
            *value = 10;
        }
        None => {
            prusti_assert!(false);
        }
    }
}

#[trusted]
fn runtime_mutation() {
    let mut map = HashMap::<Key, u32>::new();
    map.insert(Key { id: 2, ignored: 1 }, 8);
    map.insert(Key { id: 3, ignored: 1 }, 9);
    *map.get_mut(&Key { id: 2, ignored: 9 }).unwrap() = 10;
    assert_eq!(map.get(&Key { id: 2, ignored: 0 }), Some(&10));
    assert_eq!(map.get(&Key { id: 3, ignored: 0 }), Some(&9));
    assert_eq!(map.len(), 2);
    assert!(map.get_mut(&Key { id: 4, ignored: 0 }).is_none());
}

fn main() {
    noncopy_values();
    runtime_mutation();
    let mut map = HashMap::<Key, u32>::new();
    map.insert(Key { id: 2, ignored: 1 }, 8);
    map.insert(Key { id: 3, ignored: 1 }, 9);
    let equal = Key { id: 2, ignored: 9 };
    set_ten(&mut map, &equal);
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[2] == 10);
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[3] == 9);
    let len = map.len();
    prusti_assert!(len == 2);
    let missing = Key { id: 4, ignored: 0 };
    let result = map.get_mut(&missing);
    prusti_assert!(matches!(result, None));
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[2] == 10);
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[3] == 9);
    let len = map.len();
    prusti_assert!(len == 2);
}
