use prusti_contracts::hash_model::KeyProjection;
use prusti_contracts::*;
use std::collections::HashMap;

fn empty<K: KeyProjection>() {
    let map = HashMap::<K, u32>::new();
    prusti_assert!(prusti_contracts::std_spec::hash::map_contents(&map).len() == Int::from(0)); // expected-error: assertion might not hold
}

fn main() {}
