// © 2019, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(unused_must_use)]
#![allow(clippy::disallowed_types)]

#[cfg(feature = "jvm")]
mod ast_factory;
#[cfg(feature = "jvm")]
mod ast_utils;
pub mod errors;
// used by prusti-server
#[cfg(feature = "jvm")]
pub mod jni_utils;
#[macro_use]
#[cfg(feature = "jvm")]
pub mod utils;
mod cache;
// used by prusti-server
pub mod java_exception;
pub mod silicon_counterexample;
pub mod smt_manager;
mod verification_backend;
#[cfg(feature = "jvm")]
mod verification_context;
mod verification_result;
#[cfg(feature = "jvm")]
mod verifier;
#[cfg(feature = "jvm")]
mod viper;

#[cfg(feature = "jvm")]
pub use crate::{ast_factory::*, ast_utils::*, verification_context::*, verifier::*, viper::*};
pub use crate::{
    cache::*, java_exception::*, silicon_counterexample::*, verification_backend::*,
    verification_result::*,
};
