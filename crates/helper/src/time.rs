use time::format_description::FormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

pub const TIME_FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

/// Gets the current time in the local time zone and returns a formatted [String].
///
/// # Errors
///
/// Returns [`TimeInvalidOffset`](crate::Error::TimeInvalidOffset) if unable to determine the current time zone.
pub fn current_time() -> crate::Result<String> {
    let time = OffsetDateTime::now_local()?;
    let formatted = time.format(TIME_FORMAT)?;
    Ok(formatted)
}

/// Converts the timestamp into a valid date and returns a formatted [String].
///
/// # Errors
///
/// Returns [`TimeInvalidRange`](crate::Error::TimeInvalidRange) if unable to convert the timestamp to a valid date.
pub fn format_timestamp(timestamp: i64) -> crate::Result<String> {
    let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
    let formatted = time.format(TIME_FORMAT)?;
    Ok(formatted)
}

/// Returns the local offset compared to UTC. Useful for testing if the current system supports local
/// time zones, or if the program needs to operate off UTC.
/// 
/// # Errors
/// Returns a [`TimeInvalidOffset`](crate::Error::TimeInvalidOffset) if unable to get the local offset.
pub fn get_local_offset() -> crate::Result<UtcOffset> {
    Ok(UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH)?)
}
