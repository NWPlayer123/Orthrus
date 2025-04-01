//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! The contents of this module can be used by including the following in any module:
//! ```ignore
//! use orthrus_jsystem::prelude::*;
//! ```

pub use crate::error::Error;
pub(crate) use crate::error::*;
#[doc(inline)]
pub use crate::rarc2::ResourceArchive;
