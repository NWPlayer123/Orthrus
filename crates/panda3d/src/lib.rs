//! This crate contains modules for [Orthrus](https://crates.io/crates/orthrus) that add support for
//! the [Panda3D engine](https://github.com/panda3d/panda3d/).

// Here's all necessary no_std information as a nice prelude
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::string::String;
    pub use alloc::vec;
    pub use alloc::vec::Vec;
}

pub mod multifile;
pub mod subfile;

pub mod bam;

pub mod prelude;
pub mod common;

mod nodes;
