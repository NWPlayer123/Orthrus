use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimBundle {
    group: AnimGroup,
    fps: f32,
    num_frames: u16,
}

impl AnimBundle {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let group = AnimGroup::create(loader, data)?;
        let fps = data.read_float()?;
        let num_frames = data.read_u16()?;
        Ok(Self { group, fps, num_frames })
    }
}
