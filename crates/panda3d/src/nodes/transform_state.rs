use super::prelude::*;

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub(crate) struct TransformFlags: u32 {
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
pub(crate) struct TransformState {
    pub flags: TransformFlags,
    pub position: Vec3,
    /// Newer rotation that doesn't encounter a gimbal lock
    pub quaternion: Quat,
    /// Classic rotation using Heading/Pitch/Roll
    pub rotation: Vec3,
    pub scale: Vec3,
    pub shear: Vec3,
    pub matrix: Mat4,
}

impl TransformState {
    #[inline]
    fn check_uniform_scale(&mut self) {
        const EPSILON: f32 = 1.0e-6;
        if relative_eq!(self.scale.x, self.scale.y, epsilon = EPSILON)
            && relative_eq!(self.scale.x, self.scale.z, epsilon = EPSILON)
        {
            self.flags |= TransformFlags::UniformScale;
            if relative_eq!(self.scale.x, 1.0, epsilon = EPSILON) {
                self.flags |= TransformFlags::IdentityScale;
            }
        }

        if !relative_eq!(self.shear, Vec3::ZERO, epsilon = EPSILON) {
            self.flags |= TransformFlags::NonZeroShear;
        }
    }
}

impl Node for TransformState {
    #[inline]
    #[allow(clippy::field_reassign_with_default)]
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
        let mut state = Self::default();

        state.flags = TransformFlags::from_bits_truncate(data.read_u32()?);
        if state.flags.contains(TransformFlags::ComponentsGiven) {
            state.position = Vec3::read(data)?;

            if state.flags.contains(TransformFlags::QuaternionGiven) {
                state.quaternion = Quat::read(data)?;
            } else {
                state.rotation = Vec3::read(data)?;
            }

            state.scale = Vec3::read(data)?;
            state.shear = Vec3::read(data)?;

            // This needs to be called directly after reading the values
            state.check_uniform_scale();
        }

        if state.flags.contains(TransformFlags::MatrixKnown) {
            state.matrix = Mat4::read(data)?;
        }

        Ok(state)
    }
}

impl Default for TransformState {
    fn default() -> Self {
        Self {
            flags: TransformFlags::Identity
                | TransformFlags::SingularKnown
                | TransformFlags::ComponentsKnown
                | TransformFlags::HasComponents
                | TransformFlags::MatrixKnown
                | TransformFlags::QuaternionKnown
                | TransformFlags::RotationKnown
                | TransformFlags::UniformScale
                | TransformFlags::IdentityScale
                | TransformFlags::TwoDimensional
                | TransformFlags::NormalizedQuatKnown,
            position: Vec3::ZERO,
            scale: Vec3::ONE,
            shear: Vec3::ZERO,
            quaternion: Quat::IDENTITY,
            rotation: Vec3::ZERO,
            matrix: Mat4::IDENTITY,
        }
    }
}
