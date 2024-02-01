//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! For example, you can refer to [`Yaz0`], but you have to use [`yaz0::Error`].
//!
//! The contents of this module can be used by including the following in any module:
//! ```
//! use orthrus_ncompress::prelude::*;
//! ```

pub use crate::yay0::Yay0;

pub mod yay0 {
    pub use crate::yay0::{Header, CompressionAlgo, Error};
}

pub use crate::yaz0::Yaz0;

pub mod yaz0 {
    pub use crate::yaz0::{Header, CompressionAlgo, Error};
}
