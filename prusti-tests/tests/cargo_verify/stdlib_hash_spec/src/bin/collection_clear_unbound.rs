use prusti_contracts::hash_model::KeyProjection;
use prusti_contracts::*;
use std::collections::HashMap;

fn clear<K: KeyProjection>(map: &mut HashMap<K, u32>) {
    map.clear();
    prusti_assert!(prusti_contracts::std_spec::hash::map_contents(&*map).len() == Int::from(0)); // expected-error: assertion might not hold
}

fn main() {}
