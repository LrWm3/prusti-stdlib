use key_specs::Key;
use prusti_contracts::hash_model::HashKeyModel;
use prusti_contracts::*;
use std::collections::{HashMap, HashSet};

fn generic_insert<K: HashKeyModel>(map: &mut HashMap<K, u32>, key: K) {
    let projected = ghost! { key.key_model() };
    map.insert(key, 8);
    prusti_assert!(prusti_contracts::std_spec::hash::map_contents(&*map).contains(&*projected));
}

fn integer_map() {
    let mut map = HashMap::<u32, u32>::new();
    let first = map.insert(2, 8);
    prusti_assert!(matches!(first, None));
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[2] == 8);
    let previous = map.insert(2, 9);
    prusti_assert!(matches!(previous, Some(8)));
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[2] == 9);
    let len = map.len();
    prusti_assert!(len == 1);
}

fn custom_map() {
    let mut map = HashMap::<Key, u32>::new();
    let first = map.insert(Key { id: 2, ignored: 1 }, 8);
    prusti_assert!(matches!(first, None));
    let previous = map.insert(Key { id: 2, ignored: 9 }, 9);
    prusti_assert!(matches!(previous, Some(8)));
    let other = map.insert(Key { id: 3, ignored: 1 }, 10);
    prusti_assert!(matches!(other, None));
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[2] == 9);
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[3] == 10);
    let len = map.len();
    prusti_assert!(len == 2);
}

fn integer_set() {
    let mut set = HashSet::<u32>::new();
    let first = set.insert(2);
    let duplicate = set.insert(2);
    let second = set.insert(3);
    prusti_assert!(first);
    prusti_assert!(!duplicate);
    prusti_assert!(second);
    let len = set.len();
    prusti_assert!(len == 2);
}

#[trusted]
fn runtime_insertions() {
    let mut map = HashMap::new();
    assert_eq!(map.insert(Key { id: 2, ignored: 1 }, 8), None);
    assert_eq!(map.insert(Key { id: 2, ignored: 9 }, 9), Some(8));
    assert_eq!(map.insert(Key { id: 3, ignored: 1 }, 10), None);
    assert_eq!(map.len(), 2);
    assert_eq!(map[&Key { id: 2, ignored: 0 }], 9);
    assert_eq!(map[&Key { id: 3, ignored: 0 }], 10);
}

fn main() {
    let mut generic = HashMap::<u32, u32>::new();
    generic_insert(&mut generic, 4);
    integer_map();
    custom_map();
    integer_set();
    runtime_insertions();
}
