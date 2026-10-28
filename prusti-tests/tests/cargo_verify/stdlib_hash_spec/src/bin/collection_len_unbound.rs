use prusti_contracts::hash_model::KeyProjection;
use prusti_contracts::*;
use std::collections::HashMap;

fn length<K: KeyProjection>(map: &HashMap<K, u32>) {
    let actual = map.len();
    prusti_assert!(Int::from(actual) == prusti_contracts::std_spec::hash::map_contents(map).len()); // expected-error: assertion might not hold
}

fn main() {}
