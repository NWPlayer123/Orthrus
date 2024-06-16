//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! The contents of this module can be used by including the following in any module:
//! ```
//! use orthrus_jsystem::prelude::*;
//! ```

#[doc(inline)]
pub use crate::rarc::ResourceArchive;

pub mod rarc {
    #[doc(inline)]
    pub use crate::rarc::Error;
}
