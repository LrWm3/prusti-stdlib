use prusti_contracts::hash_model::KeyProjection;
use prusti_contracts::*;
use std::collections::HashMap;

fn contains<K: KeyProjection>(map: &HashMap<K, u32>, key: &K) {
    let present = map.contains_key(key);
    prusti_assert!(present == prusti_contracts::std_spec::hash::map_contents(map).contains(key.key_model())); // expected-error: assertion might not hold
}

fn main() {}
