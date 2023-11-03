use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
#[cfg(feature = "std")]
use time::UtcOffset;

pub const TIME_FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

/// Gets the current time in the local time zone and returns a formatted [String].
///
/// # Errors
///
/// Returns [`TimeInvalidOffset`](crate::Error::TimeInvalidOffset) if unable to determine the
/// current time zone.
pub fn current_time() -> time::Result<String> {
    let time = {
        #[cfg(feature = "std")]
        {
            OffsetDateTime::now_local()?
        }
        #[cfg(not(feature = "std"))]
        {
            OffsetDateTime::now_utc()
        }
    };
    let formatted = time.format(TIME_FORMAT)?;
    Ok(formatted)
}

/// Converts the timestamp into a valid date and returns a formatted [String].
///
/// # Errors
///
/// Returns [`TimeInvalidRange`](crate::Error::TimeInvalidRange) if unable to convert the timestamp
/// to a valid date.
pub fn format_timestamp(timestamp: i64) -> time::Result<String> {
    let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
    let formatted = time.format(TIME_FORMAT)?;
    Ok(formatted)
}

/// Returns the local offset compared to UTC. Useful for testing if the current system supports
/// local time zones, or if the program needs to operate off UTC.
///
/// # Errors
/// Returns a [`TimeInvalidOffset`](crate::Error::TimeInvalidOffset) if unable to get the local
/// offset.
#[cfg(feature = "std")]
pub fn get_local_offset() -> time::Result<UtcOffset> {
    Ok(UtcOffset::local_offset_at(OffsetDateTime::UNIX_EPOCH)?)
}
