//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! For example, you can refer to [`Multifile`], but you have to use [`multifile::Error`].
//!
//! The contents of this module can be used by including the following in any module:
//! ```
//! use orthrus_panda3d::prelude::*;
//! ```

pub use crate::multifile::Multifile;

pub mod multifile {
    pub use crate::multifile::{Error, Version};
}
