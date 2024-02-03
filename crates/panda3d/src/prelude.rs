//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! For example, you can refer to [`Multifile`], but you have to use [`multifile::Error`].
//!
//! The contents of this module can be used by including the following in any module:
//! ```
//! use orthrus_panda3d::prelude::*;
//! ```

#[doc(inline)]
pub use crate::multifile::Multifile;

/// Includes [`multifile::Error`] for Result handling, as well as Multifile versioning.
pub mod multifile {
    #[doc(inline)]
    pub use crate::multifile::{Error, Version};
}
