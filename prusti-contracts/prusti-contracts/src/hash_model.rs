//! Key projections and explicit evidence for modeled hash collections.

use crate::pure;
use core::hash::Hash;

/// A specification-level projection of a key to its logical identity.
///
/// This trait alone does not assert that the projection agrees with Rust
/// equality or hashing. In particular, a projection need not be injective on
/// whole-value snapshots.
pub trait KeyProjection: Eq + Hash {
    type Model;

    #[pure]
    fn key_model(&self) -> Self::Model;
}

/// Trusted evidence that a key projection models a Rust hash key.
///
/// # Safety
///
/// Each implementation is a verification assumption: Rust `Eq` must be an
/// equivalence relation and must hold exactly when the projected models are
/// equal. Equal keys must hash compatibly for every hasher started in the same
/// state. A stored key's equality, hash behavior, and projection must remain
/// stable for its entire residence in a collection, including in the presence
/// of interior mutability. The verifier does not prove these requirements from
/// the `unsafe impl`; they must be verified separately or explicitly trusted.
/// Hash collisions between unequal keys are permitted.
pub unsafe trait HashKeyModel: KeyProjection {}

mod sealed {
    pub trait Same<T: ?Sized> {}
    impl<T> Same<T> for T {}
}

/// Evidence that the borrowed lookup type is exactly the stored key type.
///
/// This trait is sealed and implemented only when both types are identical.
/// It does not establish correspondence for heterogeneous borrowed keys.
#[allow(private_bounds)]
pub trait SameKey<T: ?Sized>: sealed::Same<T> {}
impl<T> SameKey<T> for T {}

#[cfg_attr(feature = "prusti", crate::refine_trait_spec)]
impl KeyProjection for u32 {
    type Model = u32;

    #[pure]
    fn key_model(&self) -> u32 {
        *self
    }
}

// Rust's integer equality and hashing agree with this identity projection.
unsafe impl HashKeyModel for u32 {}
