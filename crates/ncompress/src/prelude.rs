//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! For example, you can refer to [`Yaz0`], but you have to use [`yaz0::Error`].
//!
//! The contents of this module can be used by including the following in any module:
//! ```ignore
//! use orthrus_ncompress::prelude::*;
//! ```

#[doc(inline)]
pub use crate::yay0::Yay0;

/// Includes [`yay0::Error`] for Result handling, [`yay0::Header`], and Yay0-specific compression
/// algorithms.
pub mod yay0 {
    #[doc(inline)]
    pub use crate::yay0::{CompressionAlgo, Error, Header};
}

#[doc(inline)]
pub use crate::yaz0::Yaz0;

/// Includes [`yaz0::Error`] for Result handling, [`yaz0::Header`], and Yaz0-specific compression
/// algorithms.
pub mod yaz0 {
    #[doc(inline)]
    pub use crate::yaz0::{CompressionAlgo, Error, Header};
}
