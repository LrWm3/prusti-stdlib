use prusti_contracts::{hash_model::KeyProjection, *};
use std::collections::HashMap;
#[requires(prusti_contracts::std_spec::hash::map_contents(&*map).contains(key.key_model()))]
fn mutate<K: KeyProjection>(map: &mut HashMap<K, u32>, key: &K) {
    if let Some(value) = map.get_mut(key) {
        *value = 10;
    }
    prusti_assert!(*prusti_contracts::std_spec::hash::map_contents(&*map)[key.key_model()] == 10);
    // expected-error: assertion might not hold
}
fn main() {}
