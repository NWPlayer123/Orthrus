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

pub mod identify;

pub mod prelude;

/// Converts a file size in bytes to a human-readable format.
/// 
/// This function condenses the length of a file until it can't be shrank any more and returns that
/// with the relevant unit (bytes, KB, MB, GB, etc). 
pub fn format_size(length: usize) -> String {
    const UNITS: [&str; 7] = ["bytes", "KB", "MB", "GB", "TB", "PB", "EB"];
    let mut size = length as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}
