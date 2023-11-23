//#![feature(const_slice_index)]

//no_std
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(not(feature = "std"))]
extern crate alloc;

//std
#[cfg(feature = "std")]
#[path = ""]
pub mod std_features {
    pub mod certificate;
    pub mod time;

    pub use time::{current_time, format_timestamp, TIME_FORMAT};
}
#[cfg(feature = "std")]
pub use crate::std_features::*;

//shared
pub mod data;
pub mod vfs;

pub mod prelude {
    pub use crate::data::{DataCursor, Endian};

    //Force library users to specify the kind of error, in this case data::error
    pub mod data {
        pub use crate::data::Error;
    }
}
pub use prelude::*;
