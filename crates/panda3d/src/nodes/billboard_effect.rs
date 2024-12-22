use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct BillboardEffect {
    pub off: bool,
    pub up_vector: Vec3,
    pub eye_relative: bool,
    pub axial_rotate: bool,
    pub offset: f32,
    pub look_at_point: Vec3,
    pub look_at: NodePath,
    pub fixed_depth: bool,
}

impl Node for BillboardEffect {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let off = data.read_bool()?;
        let up_vector = Vec3::read(data)?;
        let eye_relative = data.read_bool()?;
        let axial_rotate = data.read_bool()?;
        let offset = data.read_float()?;
        let look_at_point = Vec3::read(data)?;
        let look_at = match loader.get_minor_version() >= 43 {
            true => NodePath::create(loader, data)?,
            false => NodePath::default(),
        };
        let fixed_depth = match loader.get_minor_version() >= 43 {
            true => data.read_bool()?,
            false => false,
        };

        Ok(Self {
            off,
            up_vector,
            eye_relative,
            axial_rotate,
            offset,
            look_at_point,
            look_at,
            fixed_depth,
        })
    }
}

impl GraphDisplay for BillboardEffect {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        write!(label, "{{BillboardEffect|")?;

        // Fields
        write!(label, "off: {}|", self.off)?;
        write!(label, "up_vector: {}|", self.up_vector)?;
        write!(label, "eye_relative: {}|", self.eye_relative)?;
        write!(label, "axial_rotate: {}|", self.axial_rotate)?;
        write!(label, "offset: {}|", self.offset)?;
        write!(label, "look_at_point: {}|", self.look_at_point)?;
        write!(label, "look_at: [")?;
        let mut first = true;
        for node in &self.look_at.path_refs {
            if !first {
                write!(label, ", ")?;
            }
            write!(label, "node_{}", *node)?;
            connections.push(*node);
            first = false;
        }
        write!(label, "]|")?;
        write!(label, "fixed_depth: {}", self.fixed_depth)?;

        // Footer
        write!(label, "}}")?;
        Ok(())
    }
}
