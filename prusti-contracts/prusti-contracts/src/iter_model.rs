//! Finite iterator observations used by verification contracts.

use crate::{pure, Seq};

// Only implementations whose iterator behavior has been reviewed may expose
// the conditional `Iterator::next` contract.
pub(crate) mod sealed {
    pub trait Sealed {}
}

/// An iterator whose remaining items form a finite sequence.
///
/// For these implementations, `next` returns and removes the head of a
/// nonempty sequence. On an empty sequence it returns `None` and remains empty.
/// This ghost observation is for specifications, not executable code. The
/// capability is sealed; implementing `Iterator` alone does not provide it.
pub trait ModeledIterator: Iterator + sealed::Sealed {
    #[pure]
    fn remaining(&self) -> Seq<Self::Item>;
}

#[cfg(feature = "alloc")]
mod owned_vec {
    use super::*;
    #[cfg(feature = "prusti")]
    use crate::{refine_trait_spec, trusted};
    use alloc::vec::IntoIter;

    #[cfg(feature = "prusti")]
    impl<T, A: core::alloc::Allocator> sealed::Sealed for IntoIter<T, A> {}
    #[cfg(not(feature = "prusti"))]
    impl<T> sealed::Sealed for IntoIter<T> {}

    #[cfg(feature = "prusti")]
    #[refine_trait_spec]
    impl<T, A: core::alloc::Allocator> ModeledIterator for IntoIter<T, A> {
        #[trusted]
        #[pure]
        fn remaining(&self) -> Seq<T> {
            Seq::new()
        }
    }

    #[cfg(not(feature = "prusti"))]
    impl<T> ModeledIterator for IntoIter<T> {
        fn remaining(&self) -> Seq<T> {
            Seq::new()
        }
    }
}
