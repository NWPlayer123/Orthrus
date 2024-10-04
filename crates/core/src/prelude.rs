//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! The contents of this module can be used by including the following in any module:
//! ```
//! use orthrus_core::prelude::*;
//! ```

#[doc(inline)]
pub use crate::data::{
    ByteStream, DataCursor, DataCursorMut, DataCursorRef, Endian, EndianExt, ReadExt, SeekExt, WriteExt, DataError
};
#[doc(inline)]
pub use crate::identify::{FileIdentifier, FileInfo, IdentifyFn};

/// Includes [`util::format_size`], which allows for pretty-print of various lengths.
pub mod util {
    #[doc(inline)]
    pub use crate::util::format_size;
}

/// Includes all time functionality, for working with timestamps and the current time.
#[cfg(feature = "time")]
pub mod time {
    #[doc(inline)]
    pub use crate::time::{current_time, format_timestamp, get_local_offset};
}

/// Includes [`cert::read_certificate`], which allows for reading X.509 certificates.
#[cfg(feature = "certificate")]
pub mod cert {
    #[doc(inline)]
    pub use crate::certificate::read_certificate;
}
