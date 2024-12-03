use core::ops::{Deref, DerefMut};

use super::prelude::*;

const NUM_MATRIX_COMPONENTS: usize = 12;

// TODO: re-type this from f32 once we make read_float generic
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimChannelMatrixXfmTable {
    pub inner: AnimChannelMatrix,
    pub tables: [Vec<f32>; NUM_MATRIX_COMPONENTS],
}

impl Node for AnimChannelMatrixXfmTable {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = AnimChannelMatrix::create(loader, data)?;
        let wrote_compressed = data.read_bool()?;
        let new_hpr = data.read_bool()?;

        let mut tables: [Vec<f32>; NUM_MATRIX_COMPONENTS] = Default::default();
        if !wrote_compressed {
            for table in &mut tables {
                let table_size = data.read_u16()?;
                let mut table_data = Vec::with_capacity(table_size as usize);
                for _ in 0..table_size {
                    table_data.push(data.read_float()?);
                }
                *table = table_data;
            }

            if !new_hpr {
                unimplemented!("Haven't implemented old HPR translation in AnimChannelMatrixXfmTable");
            }
        } else {
            unimplemented!("Haven't implemented FFT decompression in AnimChannelMatrixXfmTable");
        }

        Ok(Self { inner, tables })
    }
}

impl Deref for AnimChannelMatrixXfmTable {
    type Target = AnimChannelMatrix;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AnimChannelMatrixXfmTable {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
