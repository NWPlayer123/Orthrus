//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! The contents of this module can be used by including the following in any module:
//! ```ignore
//! use orthrus_godot::prelude::*;
//! ```

#[doc(inline)]
pub use crate::pck::ResourcePack;

pub mod pck {
    #[doc(inline)]
    pub use crate::pck::Error;
}
