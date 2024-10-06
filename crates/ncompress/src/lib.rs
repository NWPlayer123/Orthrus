//! This crate contains modules for [Orthrus](https://crates.io/crates/orthrus) that add support for
//! Nintendo compression formats that are shared across multiple games or systems.

#![deny(unused_crate_dependencies)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::{format, vec};
}

// All public modules
pub mod yay0;
pub mod yaz0;

// For internal use only right now
mod algorithms;

// Prelude, for convenience
pub mod prelude;
