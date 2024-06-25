use super::prelude::*;

bitflags! {
    #[derive(Debug, Default)]
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
        const RotationGiven = 0x00400;
        const RotationKnown = 0x00800;
        const UniformScale = 0x01000;
        const IdentityScale = 0x02000;
        const NonZeroShear = 0x04000;
        const Destructing = 0x08000;
        const TwoDimensional = 0x10000;
        const NormalizedQuatKnown = 0x40000;
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct TransformState {
    flags: Flags,
    position: Vec3,
    /// Newer rotation that doesn't encounter a gimbal lock
    quaternion: Vec4,
    /// Classic rotation using Heading/Pitch/Roll
    rotation: Vec3,
    scale: Vec3,
    shear: Vec3,
    matrix: Mat4,
}

impl TransformState {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mut state = Self::default();

        state.flags = Flags::from_bits_truncate(data.read_u32()?);
        if state.flags.contains(Flags::ComponentsGiven) {
            state.position = Vec3::read(data)?;

            if state.flags.contains(Flags::QuaternionGiven) {
                state.quaternion = Vec4::read(data)?;
            } else {
                state.rotation = Vec3::read(data)?;
            }

            state.scale = Vec3::read(data)?;
            state.shear = Vec3::read(data)?;

            // This needs to be called directly after reading the values
            state.check_uniform_scale();
        }

        if state.flags.contains(Flags::MatrixKnown) {
            state.matrix = Mat4::read(data)?;
        }

        Ok(state)
    }

    #[inline]
    fn check_uniform_scale(&mut self) {
        const EPSILON: f32 = 1.0e-6;
        if relative_eq!(self.scale.x, self.scale.y, epsilon = EPSILON)
            && relative_eq!(self.scale.x, self.scale.z, epsilon = EPSILON)
        {
            self.flags |= Flags::UniformScale;
            if relative_eq!(self.scale.x, 1.0, epsilon = EPSILON) {
                self.flags |= Flags::IdentityScale;
            }
        }

        if relative_eq!(self.shear, Vec3::ZERO, epsilon = EPSILON) == false {
            self.flags |= Flags::NonZeroShear;
        }
    }
}

impl Default for TransformState {
    fn default() -> Self {
        Self {
            flags: Flags::Identity
                | Flags::SingularKnown
                | Flags::ComponentsKnown
                | Flags::HasComponents
                | Flags::MatrixKnown
                | Flags::QuaternionKnown
                | Flags::RotationKnown
                | Flags::UniformScale
                | Flags::IdentityScale
                | Flags::TwoDimensional
                | Flags::NormalizedQuatKnown,
            position: Vec3::ZERO,
            scale: Vec3::ONE,
            shear: Vec3::ZERO,
            quaternion: Vec4::X,
            rotation: Vec3::ZERO,
            matrix: Mat4::IDENTITY,
        }
    }
}
