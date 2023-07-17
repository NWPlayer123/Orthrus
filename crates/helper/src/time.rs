use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

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
