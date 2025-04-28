//! This crate contains modules for [Orthrus](https://crates.io/crates/orthrus) that add support for Nintendo
//! compression formats that are shared across multiple games or systems.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::{boxed::Box, format, vec};
}

// All public modules
pub mod lz11;
pub mod yay0;
pub mod yaz0;

// For internal use only right now
mod algorithms;

// Prelude, for convenience
pub mod prelude;
