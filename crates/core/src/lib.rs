//! This crate is used as a utilities library for common functionality across
//! [Orthrus](https://crates.io/crates/orthrus) modules.
//!
//! By default, this crate only enables modules which do not have any crate dependencies (aside from
//! snafu, which is required for errors).

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
#[allow(unused_imports)] //TODO: verify no_std again
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::String;
}

pub mod prelude;

// Enable any crates that don't have dependencies by default
pub mod data;
pub mod util;

#[cfg(feature = "std")]
pub mod identify;

// Optional crates
#[cfg(feature = "certificate")]
pub mod certificate;

#[cfg(feature = "time")]
pub mod time;

