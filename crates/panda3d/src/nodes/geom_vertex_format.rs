use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexFormat {
    animation: GeomVertexAnimationSpec,
    /// References to all GeomVertexArrayFormat data
    arrays: Vec<u32>,
}

impl GeomVertexFormat {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let animation = GeomVertexAnimationSpec::create(loader, data)?;

        let num_arrays = data.read_u16()?;
        let mut arrays = Vec::with_capacity(num_arrays as usize);
        for _ in 0..num_arrays {
            arrays.push(loader.read_pointer(data)?.unwrap());
        }

        Ok(Self { animation, arrays })
    }
}
