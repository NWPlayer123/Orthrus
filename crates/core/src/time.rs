//! Utility module for working with timestamps and getting the current time.

//re-export time::Error since we use it, so other libraries can implement From<time::Error>
pub use time::Error;
use time::{OffsetDateTime, UtcOffset};

#[cfg(not(feature = "std"))]
use crate::no_std::*;

/// Convert a timestamp into a formatted [`String`].
#[cfg(feature = "alloc")]
#[inline]
pub fn format_timestamp(timestamp: i64) -> time::Result<String> {
    let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.year(),
        time.month() as u8,
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    ))
}

/// Get the current time as a Unix timestamp (seconds since the Unix epoch).
#[cfg(feature = "std")]
#[inline]
pub fn current_timestamp() -> time::Result<i64> {
    Ok(OffsetDateTime::now_local()?.unix_timestamp())
}

/// Returns a formatted [String] with the current time.
#[cfg(feature = "std")]
#[inline]
pub fn current_time() -> time::Result<String> {
    let time = OffsetDateTime::now_local()?;
    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.year(),
        time.month() as u8,
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    ))
}

/// Returns the local time zone offset.
///
/// This is useful for testing if the current system has a local offset or if only UTC is available.
#[cfg(feature = "std")]
#[inline]
pub fn local_offset() -> time::Result<UtcOffset> {
    Ok(OffsetDateTime::now_local()?.offset())
}
