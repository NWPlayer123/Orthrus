//#![feature(const_slice_index)]

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
mod no_std {
    extern crate alloc;
    pub use alloc::boxed::Box;
    pub use alloc::vec;
}

//Always have data enabled
pub mod data;
//These are all set behind feature flags
#[cfg(all(feature = "time", feature = "std"))]
pub mod time;

pub mod prelude {
    pub use crate::data::{DataCursor, Endian};

    //Force library users to specify the kind of error, in this case data::error
    pub mod data {
        pub use crate::data::Error;
    }

    #[cfg(all(feature = "time", feature = "std"))]
    pub mod time {
        pub use crate::time::*;
    }
}
pub use prelude::*;
