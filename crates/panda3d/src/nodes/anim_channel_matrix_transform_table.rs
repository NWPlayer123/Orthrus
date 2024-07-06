use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimChannelMatrixXfmTable {
    matrix: AnimChannelMatrix,
    tables: [Vec<f32>; 12],
}

impl AnimChannelMatrixXfmTable {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let matrix = AnimChannelMatrix::create(loader, data)?;
        let wrote_compressed = data.read_bool()?;
        let new_hpr = data.read_bool()?;

        let mut tables: [Vec<f32>; 12] = Default::default();
        if !wrote_compressed {
            for n in 0..12 {
                let table_size = data.read_u16()?;
                let mut table = Vec::with_capacity(table_size as usize);
                for _ in 0..table_size {
                    table.push(data.read_float()?);
                }
                tables[n] = table;
            }

            if !new_hpr {
                panic!("Haven't implemented HPR translation in AnimChannelMatrixXfmTable");
            }
        } else {
            panic!("Haven't implemented animation decompression in AnimChannelMatrixXfmTable");
        }
        
        Ok(Self { matrix, tables })
    }
}
