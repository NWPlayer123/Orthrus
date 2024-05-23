use approx::relative_eq;
use bitflags::bitflags;
use nalgebra::{matrix, point, vector, Matrix4, Point3, Vector3, Vector4};

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
    position: Point3<f64>,
    quaternion: Vector4<f64>,
    rotation: Vector3<f64>,
    scale: Vector3<f64>,
    shear: Vector3<f64>,
    matrix: Matrix4<f64>,
}

impl TransformState {
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mut state = Self::default();

        state.flags = Flags::from_bits_truncate(data.read_u32()?);
        if state.flags.contains(Flags::ComponentsGiven) {
            state.position = point![data.read_float()?, data.read_float()?, data.read_float()?];

            if state.flags.contains(Flags::QuaternionGiven) {
                state.quaternion = vector![
                    data.read_float()?,
                    data.read_float()?,
                    data.read_float()?,
                    data.read_float()?,
                ];
            } else {
                // Heading, Pitch, Roll
                state.rotation =
                    vector![data.read_float()?, data.read_float()?, data.read_float()?];
            }
            state.scale = vector![data.read_float()?, data.read_float()?, data.read_float()?];
            state.shear = vector![data.read_float()?, data.read_float()?, data.read_float()?];

            // This needs to be called directly after reading the values
            state.check_uniform_scale();
        }

        if state.flags.contains(Flags::MatrixKnown) {
            //TODO: make this less awful lol
            state.matrix = matrix![
                data.read_float()?,
                data.read_float()?,
                data.read_float()?,
                data.read_float()?;
                data.read_float()?,
                data.read_float()?,
                data.read_float()?,
                data.read_float()?;
                data.read_float()?,
                data.read_float()?,
                data.read_float()?,
                data.read_float()?;
                data.read_float()?,
                data.read_float()?,
                data.read_float()?,
                data.read_float()?;
            ];
        }

        Ok(state)
    }

    fn check_uniform_scale(&mut self) {
        if relative_eq!(self.scale[0], self.scale[1], epsilon = 1.0e-6)
            && relative_eq!(self.scale[0], self.scale[2], epsilon = 1.0e-6)
        {
            self.flags |= Flags::UniformScale;
            if relative_eq!(self.scale[0], 1.0, epsilon = 1.0e-6) {
                self.flags |= Flags::IdentityScale;
            }
        }

        if relative_eq!(self.shear, Vector3::zeros(), epsilon = 1.0e-6) {
            self.flags |= Flags::NonZeroShear;
        }
    }
}
