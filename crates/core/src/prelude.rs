//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! For example, when working with [`DataCursor`], you have to explicitly refer to [`data::Error`].
//!
//! The contents of this module can be used by including the following in any module:
//! ```
//! use orthrus_core::prelude::*;
//! ```

#[doc(inline)]
pub use crate::data::{
    DataCursor, DataCursorMut, DataCursorRef, DataCursorTrait, Endian, EndianRead, EndianWrite,
};
pub use crate::identify::*;

/// Contains [`data::Error`], which is used in Results returned by [`DataCursor`]
pub mod data {
    pub use crate::data::Error;
}

#[cfg(all(feature = "time", feature = "std"))]
pub mod time {
    pub use crate::time::*;
}

pub mod util {
    pub use crate::format_size;
}
