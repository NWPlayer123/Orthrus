use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct AnimBundle {
    pub group: AnimGroup,
    pub fps: f32,
    pub num_frames: u16,
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
