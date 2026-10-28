use key_specs::Key;
use prusti_contracts::hash_model::HashKeyModel;
use prusti_contracts::*;
use std::collections::{HashMap, HashSet};

fn empty<K: HashKeyModel>() {
    let map = HashMap::<K, u32>::new();
    let set = HashSet::<K>::new();
    prusti_assert!(prusti_contracts::std_spec::hash::map_contents(&map).len() == Int::from(0));
    prusti_assert!(prusti_contracts::std_spec::hash::set_contents(&set).len() == Int::from(0));
}

fn main() {
    empty::<u32>();
    empty::<Key>();
}
