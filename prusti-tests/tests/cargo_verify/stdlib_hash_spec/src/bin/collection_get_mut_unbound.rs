use prusti_contracts::hash_model::KeyProjection;
use prusti_contracts::*;
use std::collections::HashMap;

#[requires(prusti_contracts::std_spec::hash::map_contents(&*map).contains(key.key_model()))]
#[requires(*prusti_contracts::std_spec::hash::map_contents(&*map)[key.key_model()] == 8)]
fn lookup<K: KeyProjection>(map: &mut HashMap<K, u32>, key: &K) {
    if let Some(value) = map.get_mut(key) {
        prusti_assert!(*value == 8); // expected-error: assertion might not hold
    }
}

fn main() {}
