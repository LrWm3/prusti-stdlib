//! Trusted logical observations of standard hash collections.
//!
//! A map observation records value snapshots by projected key; a set
//! observation records projected-key membership. The observations do not
//! expose the stored representative of an equality class or the hash values.

use crate::{
    hash_model::{HashKeyModel, KeyProjection, SameKey},
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

#[extern_spec]
impl<K, V, S, A: Allocator> HashMap<K, V, S, A> {
    #[trusted]
    #[refine_spec(where K: HashKeyModel, [
        ensures(Int::from(result) == map_contents(self).len())
    ])]
    fn len(&self) -> usize;
}

#[extern_spec]
impl<K, S, A: Allocator> HashSet<K, S, A> {
    #[trusted]
    #[refine_spec(where K: HashKeyModel, [
        ensures(Int::from(result) == set_contents(self).len())
    ])]
    fn len(&self) -> usize;
}

#[extern_spec]
impl<K: Eq + core::hash::Hash, V, S: core::hash::BuildHasher, A: Allocator> HashMap<K, V, S, A> {
    #[trusted]
    #[refine_spec(where K: HashKeyModel, [
        ensures(map_contents(self) == old(map_contents(&*self)).insert(k.key_model(), v)),
        ensures(match &result {
            Some(previous) => old(map_contents(&*self)).contains(k.key_model())
                && *previous === *old(map_contents(&*self))[k.key_model()],
            None => !old(map_contents(&*self)).contains(k.key_model()),
        })
    ])]
    fn insert(&mut self, k: K, v: V) -> Option<V>;
}

#[extern_spec]
impl<K: Eq + core::hash::Hash, S: core::hash::BuildHasher, A: Allocator> HashSet<K, S, A> {
    #[trusted]
    #[refine_spec(where K: HashKeyModel, [
        ensures(set_contents(self) == old(set_contents(&*self)).union(Set::single(value.key_model()))),
        ensures(result == !old(set_contents(&*self)).contains(value.key_model()))
    ])]
    fn insert(&mut self, value: K) -> bool;
}

#[extern_spec]
impl<K: Eq + core::hash::Hash, V, S: core::hash::BuildHasher, A: Allocator> HashMap<K, V, S, A> {
    #[trusted]
    #[refine_spec(where K: HashKeyModel + SameKey<Q>, Q: KeyProjection<Model = K::Model>, [
        ensures(result == map_contents(self).contains(k.key_model()))
    ])]
    fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq;

    #[trusted]
    #[refine_spec(where K: HashKeyModel + SameKey<Q>, Q: KeyProjection<Model = K::Model>, [
        ensures(match result {
            Some(value) => map_contents(self).contains(k.key_model())
                && *value === *map_contents(self)[k.key_model()],
            None => !map_contents(self).contains(k.key_model()),
        })
    ])]
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq;
}

#[extern_spec]
impl<K: Eq + core::hash::Hash, S: core::hash::BuildHasher, A: Allocator> HashSet<K, S, A> {
    #[trusted]
    #[refine_spec(where K: HashKeyModel + SameKey<Q>, Q: KeyProjection<Model = K::Model>, [
        ensures(result == set_contents(self).contains(value.key_model()))
    ])]
    fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq;
}
