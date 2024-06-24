use approx::relative_eq;
use bitflags::bitflags;
use glam::{dmat4, dvec3, dvec4, DMat4, DVec3, DVec4};

use super::prelude::*;

bitflags! {
    #[derive(Default, Debug)]
    pub(crate) struct Flags: u32 {
        const Identity = 0x00001;
        const Singular = 0x00002;
        const SingularKnown = 0x00004;
        const ComponentsGiven = 0x00008;
        const ComponentsKnown = 0x00010;
        const HasComponents = 0x00020;
        const MatrixKnown = 0x00040;
        const Invalid = 0x00080;
        const QuaternionGiven = 0x00100;
        const QuaternionKnown = 0x00200;
        const HprGiven = 0x00400;
        const HprKnown = 0x00800;
        const UniformScale = 0x01000;
        const IdentityScale = 0x02000;
        const NonZeroShear = 0x04000;
        const Destructing = 0x08000;
        const TwoDimensional = 0x10000;
        const NormalizedQuaternionKnown = 0x40000;
    }
}

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct TransformState {
    flags: Flags,
    position: DVec3,
    quaternion: DVec4,
    rotation: DVec3,
    scale: DVec3,
    shear: DVec3,
    matrix: DMat4,
}

impl TransformState {
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mut state = Self::default();

        state.flags = Flags::from_bits_truncate(data.read_u32()?);
        if state.flags.contains(Flags::ComponentsGiven) {
            state.position = DVec3::read(data)?;

            if state.flags.contains(Flags::QuaternionGiven) {
                state.quaternion = DVec4::read(data)?;
            } else {
                // Heading, Pitch, Roll
                state.rotation = DVec3::read(data)?;
            }
            state.scale = DVec3::read(data)?;
            state.shear = DVec3::read(data)?;

            // This needs to be called directly after reading the values
            state.check_uniform_scale();
        }

        if state.flags.contains(Flags::MatrixKnown) {
            state.matrix = DMat4::read(data)?;
        }

        Ok(state)
    }

    fn check_uniform_scale(&mut self) {
        const EPSILON: f64 = 1.0e-6;
        if relative_eq!(self.scale.x, self.scale.y, epsilon = EPSILON)
            && relative_eq!(self.scale[0], self.scale[2], epsilon = EPSILON)
        {
            self.flags |= Flags::UniformScale;
            if relative_eq!(self.scale[0], 1.0, epsilon = EPSILON) {
                self.flags |= Flags::IdentityScale;
            }
        }

        if relative_eq!(self.shear.x, DVec3::ZERO.x, epsilon = EPSILON)
            && relative_eq!(self.shear.y, DVec3::ZERO.y, epsilon = EPSILON)
            && relative_eq!(self.shear.z, DVec3::ZERO.z, epsilon = EPSILON)
        {
            self.flags |= Flags::NonZeroShear;
        }
    }
}
