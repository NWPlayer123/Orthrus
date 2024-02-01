//! Utility module for working with timestamps and getting the current time.

#[cfg(not(feature = "std"))]
use crate::no_std::*;

//re-export time::Error since we use it, so other libraries can implement From<time::Error>
pub use time::Error;
use time::{OffsetDateTime, UtcOffset};

/// Returns a formatted [String] with the current time.
/// 
/// Note that this may be the local time, or may be based off UTC. If it matters, check whether
/// [`get_local_offset`] returns an error.
#[must_use]
#[inline]
pub fn current_time() -> String {
    let time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    format!("{}-{}-{} {}:{}:{}", time.year(), time.month() as u8, time.day(), time.hour(), time.minute(), time.second())
}

/// Convert a timestamp into a formatted [`String`].
///
/// # Errors
/// Returns [`ComponentRange`](time::Error::ComponentRange) if unable to convert the timestamp to a
/// valid date.
#[inline]
pub fn format_timestamp(timestamp: i64) -> time::Result<String> {
    let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
    Ok(format!("{}-{}-{} {}:{}:{}", time.year(), time.month() as u8, time.day(), time.hour(), time.minute(), time.second()))
}

/// Returns the local time zone offset. This is useful for testing if the current system supports
/// local time, or only UTC is available.
///
/// # Errors
/// Returns [`IndeterminateOffset`](time::Error::IndeterminateOffset) if unable to determine the
/// current time zone.
#[inline]
pub fn get_local_offset() -> time::Result<UtcOffset> {
    Ok(UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH)?)
}
