use downcast_rs::{impl_downcast, DowncastSync};

use super::prelude::*;

pub trait Node: Send + DowncastSync + core::fmt::Debug {
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error>
    where
        Self: Sized;
}

impl_downcast!(sync Node);
