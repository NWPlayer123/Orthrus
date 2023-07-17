use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

pub const TIME_FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

pub fn current_time() -> crate::Result<String> {
    let time = OffsetDateTime::now_local()?;
    let formatted = time.format(TIME_FORMAT)?;
    Ok(formatted)
}

pub fn format_timestamp(timestamp: i64) -> crate::Result<String> {
    let time = OffsetDateTime::from_unix_timestamp(timestamp)?;
    let formatted = time.format(TIME_FORMAT)?;
    Ok(formatted)
}
