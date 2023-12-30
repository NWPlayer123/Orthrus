//! This crate is used as a utilities library for common functionality across
//! [Orthrus](https://crates.io/crates/orthrus) modules.
//!
//! By default, this crate only enables the data module, which contains
//! [`DataCursor`](data::DataCursor), [`DataCursorRef`](data::DataCursorRef), and
//! [`DataCursorMut`](data::DataCursorMut), the owned and unowned read-only/read-write variants for
//! manipulating endian-specific data and complex files.
//!
//! Additionally, there is a time module, which provides convenient functions for handling
//! timestamps and getting the current time, and a certificate module, which provides a custom X509
//! implementation to allow for reading of consecutive X.509 certificates.

//#![feature(const_slice_index)]

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::vec;
}

//Always have data enabled
pub mod data;
//These are all set behind feature flags
#[cfg(all(feature = "time", feature = "std"))]
pub mod time;

#[cfg(feature = "certificate")]
pub mod certificate;

pub mod prelude;
