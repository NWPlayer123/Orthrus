use crate::bam;
use crate::bam::BinaryAsset;
use crate::common::Datagram;
use orthrus_core::prelude::*;

use super::dispatch::{DatagramRead, DatagramWrite};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
enum AnimationType {
    #[default]
    /// No vertex animation performed.
    None,
    /// Animations are processed on the CPU through Panda3D.
    Panda,
    /// Animations are hardware-accelerated on the GPU.
    Hardware,
}

impl TryFrom<u8> for AnimationType {
    type Error = bam::Error;

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => AnimationType::None,
            1 => AnimationType::Panda,
            2 => AnimationType::Hardware,
            _ => return Err(bam::Error::InvalidEnum),
        })
    }
}

#[derive(Default, Debug)]
pub(crate) struct GeomVertexAnimationSpec {
    animation_type: AnimationType,
    num_transforms: u16,
    indexed_transforms: bool,
}

impl GeomVertexAnimationSpec {
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, crate::bam::Error> {
        let animation_type: AnimationType = data.read_u8()?.try_into()?;
        let num_transforms = data.read_u16()?;
        let indexed_transforms = data.read_bool()?;

        Ok(Self {
            animation_type,
            num_transforms,
            indexed_transforms,
        })
    }
}

impl DatagramRead for GeomVertexAnimationSpec {
    fn finalize(&self) -> Result<(), crate::bam::Error> {
        Ok(())
    }
}

impl DatagramWrite for GeomVertexAnimationSpec {
    fn write(&self) -> Result<Datagram, crate::bam::Error> {
        Err(bam::Error::EndOfFile)
    }
}

impl core::fmt::Display for GeomVertexAnimationSpec {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.animation_type {
            AnimationType::None => write!(f, "none"),
            AnimationType::Panda => write!(f, "panda"),
            AnimationType::Hardware => write!(
                f,
                "hardware({}, {})",
                self.num_transforms, self.indexed_transforms
            ),
        }
    }
}
