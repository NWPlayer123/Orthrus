use time::format_description::FormatItem;
use time::macros::format_description;
//re-export time::Error since we use it directly so other libraries can implement
// From<time::Error>
pub use time::Error;
use time::{OffsetDateTime, UtcOffset};

pub const TIME_FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

/// This function will return a formatted [String] of the current local time.
///
/// # Errors
/// Returns [`IndeterminateOffset`](time::Error::IndeterminateOffset) if unable to determine the
/// current time zone.
#[must_use]
#[inline]
pub fn current_time() -> String {
    let time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    //Theoretically this should never fail
    time.format(TIME_FORMAT).unwrap()
}

/// This function tries to convert a timestamp into a formatted [String].
///
/// # Errors
/// Returns [`ComponentRange`](time::Error::ComponentRange) if unable to convert the timestamp to a
/// valid date.
#[inline]
pub fn format_timestamp(timestamp: i64) -> time::Result<String> {
    let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
    //Theoretically this should never fail
    let formatted = time.format(TIME_FORMAT).unwrap();
    Ok(formatted)
}

/// This function tries to return the time zone offset. This is useful for testing if the current
/// system supports local time, or if we can only use UTC.
///
/// # Errors
/// Returns [`IndeterminateOffset`](time::Error::IndeterminateOffset) if unable to determine the
/// current time zone.
#[inline]
pub fn get_local_offset() -> time::Result<UtcOffset> {
    Ok(UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH)?)
}
