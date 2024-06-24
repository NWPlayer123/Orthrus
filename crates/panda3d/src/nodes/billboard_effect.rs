use glam::DVec3;

use super::node_path::NodePath;
use super::prelude::*;

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct BillboardEffect {
    off: bool,
    up_vector: DVec3,
    eye_relative: bool,
    axial_rotate: bool,
    offset: f64,
    look_at_point: DVec3,
    look_at: NodePath,
    fixed_depth: bool,
}

impl BillboardEffect {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let off = data.read_bool()?;
        let up_vector = DVec3::read(data)?;
        let eye_relative = data.read_bool()?;
        let axial_rotate = data.read_bool()?;
        let offset: f64 = data.read_float()?;
        let look_at_point = DVec3::read(data)?;

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
