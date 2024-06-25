use super::prelude::*;

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct BillboardEffect {
    off: bool,
    up_vector: Vec3,
    eye_relative: bool,
    axial_rotate: bool,
    offset: f32,
    look_at_point: Vec3,
    look_at: NodePath,
    fixed_depth: bool,
}

impl BillboardEffect {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let off = data.read_bool()?;
        let up_vector = Vec3::read(data)?;
        let eye_relative = data.read_bool()?;
        let axial_rotate = data.read_bool()?;
        let offset = data.read_float()?;
        let look_at_point = Vec3::read(data)?;

        let mut effect = Self {
            off,
            up_vector,
            eye_relative,
            axial_rotate,
            offset,
            look_at_point,
            ..Default::default()
        };

        if loader.get_minor_version() >= 43 {
            effect.look_at = NodePath::create(loader, data)?;
            effect.fixed_depth = data.read_bool()?;
        }

        Ok(effect)
    }
}
