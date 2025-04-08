use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct SparseArray {
    pub subranges: Vec<(i32, i32)>,
    pub inverse: bool,
}

impl SparseArray {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_subranges = data.read_u32()?;
        let mut subranges = Vec::with_capacity(num_subranges as usize);
        for _ in 0..num_subranges {
            //TODO: use actual ranges?
            let begin = data.read_i32()?;
            let end = data.read_i32()?;
            subranges.push((begin, end));
        }

        let inverse = data.read_bool()?;

        Ok(Self { subranges, inverse })
    }
}

impl GraphDisplay for SparseArray {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        write!(label, "{{SparseArray|")?;

        // Fields
        write!(label, "ranges: [")?;
        let mut first = true;
        for range in &self.subranges {
            if !first {
                write!(label, ", ")?;
            }
            write!(label, "({}, {})", range.0, range.1)?;
            first = false;
        }
        write!(label, "]|")?;
        write!(label, "inverse: {}", self.inverse)?;

        // Footer
        write!(label, "}}")?;
        Ok(())
    }
}
