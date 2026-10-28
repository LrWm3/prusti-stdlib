//! Trusted logical observations of standard hash collections.
//!
//! A map observation records value snapshots by projected key; a set
//! observation records projected-key membership. The observations do not
//! expose the stored representative of an equality class or the hash values.

use crate::{
    hash_model::{HashKeyModel, KeyProjection},
    *,
};
use core::alloc::Allocator;
use std::collections::{HashMap, HashSet};

/// Observe the logical contents of a hash map. This is a proof-only operation.
#[trusted]
#[pure]
pub fn map_contents<K: KeyProjection, V, S, A: Allocator>(
    _: &HashMap<K, V, S, A>,
) -> Map<K::Model, V> {
    Map::new()
}

/// Observe the logical membership of a hash set. This is a proof-only operation.
#[trusted]
#[pure]
pub fn set_contents<K: KeyProjection, S, A: Allocator>(_: &HashSet<K, S, A>) -> Set<K::Model> {
    Set::new()
}

#[extern_spec]
impl<K, V> HashMap<K, V> {
    #[trusted]
    #[refine_spec(where K: HashKeyModel, [ensures(map_contents(&result) == Map::new())])]
    fn new() -> Self;
}

#[extern_spec]
impl<K> HashSet<K> {
    #[trusted]
    #[refine_spec(where K: HashKeyModel, [ensures(set_contents(&result) == Set::new())])]
    fn new() -> Self;
}
