//#![feature(const_slice_index)]
pub mod certificate;
pub mod data;
pub mod error;
pub mod time;
pub mod vfs;
pub use crate::time::{current_time, format_timestamp, TIME_FORMAT};

pub mod prelude {
    pub use crate::data::{DataCursor, Endian};
    pub use crate::error::{Error, Result};
}

pub use prelude::*;
