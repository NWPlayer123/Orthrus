// Here's all necessary no_std information as a nice prelude
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::string::String;
    pub use alloc::vec;
}

pub mod multifile;

pub mod prelude;
