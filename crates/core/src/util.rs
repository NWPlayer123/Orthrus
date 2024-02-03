//! Utility functions that can't be grouped into any other module.

#[cfg(not(feature = "std"))]
use crate::no_std::*;

/// Converts a file size in bytes to a human-readable format.
///
/// This function condenses the length of a file until it can't be shrank any more and returns that
/// with the relevant unit (bytes, KB, MB, GB, etc).
/// 
/// # Warnings
/// This function uses f64, which on a 64-bit system will lose precision if the length is too large,
/// but it should still round to a close-enough value.
#[must_use]
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
