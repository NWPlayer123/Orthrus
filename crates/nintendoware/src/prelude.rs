//! Convenient re-exports of commonly used data types, designed to make crate usage painless.
//!
//! The contents of this module can be used by including the following in any module:
//! ```
//! use orthrus_nintendoware::prelude::*;
//! ```

#[expect(non_snake_case)]
pub mod Wii {
    #[doc(inline)]
    pub use crate::rvl::stream::StreamFile;
}


#[expect(non_snake_case)]
pub mod Switch {
    #[doc(inline)]
    pub use crate::switch::BFSAR;
}
