//! This crate contains modules for [Orthrus](https://crates.io/crates/orthrus) that add support for
//! the [Panda3D engine](https://github.com/panda3d/panda3d/).

#![deny(unused_crate_dependencies)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::collections::BTreeMap;
    pub use alloc::string::String;
    pub use alloc::vec::Vec;
}

pub mod multifile;
pub mod subfile;

pub mod bam;
//#[cfg(feature = "bevy")]
//pub mod bevy;
#[cfg(feature = "bevy")]
pub mod bevy2;

pub mod common;
pub mod prelude;

mod nodes;

pub mod bam2;

pub mod multifile2;
