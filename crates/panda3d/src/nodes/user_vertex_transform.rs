use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct UserVertexTransform {
    matrix: Mat4,
}

impl Node for UserVertexTransform {
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { matrix: Mat4::read(data)? })
    }
}

impl GraphDisplay for UserVertexTransform {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{UserVertexTransform|")?;
        }

        // Fields
        write!(
            label,
            "{{matrix|{}\\n{}\\n{}\\n{}}}",
            self.matrix.w_axis, self.matrix.x_axis, self.matrix.y_axis, self.matrix.z_axis
        )?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
