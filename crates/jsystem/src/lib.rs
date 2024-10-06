//! This crate contains modules for [Orthrus](https://crates.io/crates/orthrus) that add support for
//! the JSystem framework used in multiple first-party Nintendo games on GameCube and Wii.

#![deny(unused_crate_dependencies)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::{format, vec};
}

pub mod prelude;
pub mod rarc;
