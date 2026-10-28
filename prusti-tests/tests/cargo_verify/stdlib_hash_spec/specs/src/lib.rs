#![no_std]

use core::hash::{Hash, Hasher};
use prusti_contracts::*;

// This proof interface describes its own `equivalent` operation. It does not
// yet promise that arbitrary implementations agree with Rust's `Eq` or `Hash`.
pub trait Projection: Eq + Hash {
    type Model;

    #[pure]
    fn project(&self) -> Self::Model;

    #[pure]
    #[ensures(result == (self.project() === other.project()))]
    fn equivalent(&self, other: &Self) -> bool;
}

pub struct Key {
    pub id: u32,
    pub ignored: u32,
}

impl Key {
    #[pure]
    pub fn hash_model(&self) -> u32 {
        0
    }
}

#[refine_trait_spec]
impl PartialEq for Key {
    #[pure]
    #[ensures(result == (self.id == other.id))]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Key {}

// Reviewed assumption for this probe: hashing always writes the same byte.
// This method is trusted because the generic Hasher implementation is opaque.
#[refine_trait_spec]
impl Hash for Key {
    #[trusted]
    fn hash<H: Hasher>(&self, state: &mut H) {
        0u8.hash(state);
    }
}

#[refine_trait_spec]
impl Projection for Key {
    type Model = u32;

    #[pure]
    fn project(&self) -> u32 {
        self.id
    }

    #[pure]
    fn equivalent(&self, other: &Self) -> bool {
        self == other
    }
}

#[refine_trait_spec]
impl Projection for u32 {
    type Model = u32;

    #[pure]
    fn project(&self) -> u32 {
        *self
    }

    #[pure]
    fn equivalent(&self, other: &Self) -> bool {
        *self == *other
    }
}

#[pure]
#[ensures(result == (left.project() === right.project()))]
pub fn equivalent<K: Projection>(left: &K, right: &K) -> bool {
    left.equivalent(right)
}

#[pure]
#[ensures(result.contains(key.project()))]
#[ensures(*result[key.project()] == value)]
pub fn model_insert<K: Projection>(
    map: Map<K::Model, u32>,
    key: &K,
    value: u32,
) -> Map<K::Model, u32> {
    map.insert(key.project(), value)
}

pub trait Guarded {
    #[pure]
    #[requires(input > 0)]
    fn apply(&self, input: u32) -> u32;
}

pub struct Identity;

#[refine_trait_spec]
impl Guarded for Identity {
    #[pure]
    fn apply(&self, input: u32) -> u32 {
        input
    }
}

#[refine_trait_spec]
impl prusti_contracts::hash_model::KeyProjection for Key {
    type Model = u32;

    #[pure]
    fn key_model(&self) -> u32 {
        self.id
    }
}

// Reviewed for this fixture: `PartialEq` compares id, the hash is constant,
// and the fields cannot change while the key is stored.
unsafe impl prusti_contracts::hash_model::HashKeyModel for Key {}

// Deliberately not a modeled borrowed-key correspondence: the hash of this
// projection is not assumed to match Key's constant hash.
impl core::borrow::Borrow<u32> for Key {
    fn borrow(&self) -> &u32 {
        &self.id
    }
}
