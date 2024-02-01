//! This crate is used as a utilities library for common functionality across
//! [Orthrus](https://crates.io/crates/orthrus) modules.
//!
//! By default, this crate only enables modules which do not have any crate dependencies (aside from
//! snafu, which is required for errors).

//#![feature(const_slice_index)]

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::string::String;
    pub use alloc::boxed::Box;
    pub use alloc::format;
}

//Always have data enabled
pub mod data;

#[cfg(feature = "certificate")]
pub mod certificate;

#[cfg(feature = "time")]
pub mod time;

pub mod identify;

pub mod util;

pub mod prelude;
