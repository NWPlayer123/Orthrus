//! This crate contains modules for [Orthrus](https://crates.io/crates/orthrus) that add support for
//! the JSystem framework used in multiple first-party Nintendo games on GameCube and Wii.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::{boxed::Box, format, vec};
}

pub mod error;
pub mod prelude;
pub mod rarc;
pub mod rarc2;
