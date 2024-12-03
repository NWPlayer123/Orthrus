use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct TransformBlendTable {
    pub blends: Vec<TransformBlend>,
    pub rows: SparseArray,
}

impl Node for TransformBlendTable {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_blends = data.read_u16()?;
        let mut blends = Vec::with_capacity(num_blends as usize);
        for _ in 0..num_blends {
            blends.push(TransformBlend::create(loader, data)?);
        }

        if loader.get_minor_version() < 7 {
            unimplemented!("I don't have any BAM files this old - message me");
        }
        let rows = SparseArray::create(loader, data)?;

        //There is cdata but it doesn't actually have any BAM data stored
        Ok(Self { blends, rows })
    }
}
