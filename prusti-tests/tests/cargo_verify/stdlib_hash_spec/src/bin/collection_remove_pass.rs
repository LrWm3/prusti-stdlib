use key_specs::Key;
use prusti_contracts::*;
use std::collections::{HashMap, HashSet};

fn main() {
    let mut map = HashMap::<Key, u32>::new();
    map.insert(Key { id: 2, ignored: 1 }, 8);
    map.insert(Key { id: 3, ignored: 1 }, 9);
    let previous = map.remove(&Key { id: 2, ignored: 9 });
    prusti_assert!(matches!(previous, Some(8)));
    prusti_assert!(!prusti_contracts::std_spec::hash::map_contents(&map).contains(2));
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&map)[3] == 9);
    let len = map.len();
    prusti_assert!(len == 1);
    let absent = map.remove(&Key { id: 2, ignored: 0 });
    prusti_assert!(matches!(absent, None));
    let len = map.len();
    prusti_assert!(len == 1);
    map.clear();
    let empty = map.is_empty();
    prusti_assert!(empty);
    let len = map.len();
    prusti_assert!(len == 0);

    let mut set = HashSet::<Key>::new();
    set.insert(Key { id: 2, ignored: 1 });
    set.insert(Key { id: 3, ignored: 1 });
    let removed = set.remove(&Key { id: 2, ignored: 9 });
    prusti_assert!(removed);
    prusti_assert!(!prusti_contracts::std_spec::hash::set_contents(&set).contains(2));
    prusti_assert!(prusti_contracts::std_spec::hash::set_contents(&set).contains(3));
    let len = set.len();
    prusti_assert!(len == 1);
    let absent = set.remove(&Key { id: 2, ignored: 0 });
    prusti_assert!(!absent);
    set.clear();
    let empty = set.is_empty();
    prusti_assert!(empty);
}
