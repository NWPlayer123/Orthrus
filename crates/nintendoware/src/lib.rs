//! This crate contains modules for [Orthrus](https://crates.io/crates/orthrus) that add support for
//! the NintendoWare development middleware.

// Here's all necessary no_std information as a nice prelude
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::{format, vec};
}

// All public modules
pub mod error;
pub mod switch;

// Prelude, for convenience
pub mod prelude;

mod rvl;