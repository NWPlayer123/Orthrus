//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! For example, you can refer to [`Multifile`], but you have to use [`multifile::Error`].
//!
//! The contents of this module can be used by including the following in any module:
//! ```ignore
//! use orthrus_panda3d::prelude::*;
//! ```

#[doc(inline)] pub use crate::multifile::Multifile;

/// Includes [`multifile::Error`] for Result handling.
pub mod multifile {
    #[doc(inline)] pub use crate::multifile::Error;
}

#[doc(inline)] pub use crate::bam::BinaryAsset;

/// Includes [`bam::Error`] for Result handling.
pub mod bam {
    #[doc(inline)] pub use crate::bam::Error;
}

/// Includes [`panda3d::Version`] for file format versions.
pub mod panda3d {
    #[doc(inline)] pub use crate::common::Version;
}
