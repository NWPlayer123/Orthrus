use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct SparseArray {
    subranges: Vec<(i32, i32)>,
    inverse: bool,
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
