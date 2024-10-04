use super::prelude::*;

// This is technically a generic but I don't feel like making one
#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct AnimChannelMatrix {
    pub group: AnimGroup,
    pub last_frame: u16,
}

impl AnimChannelMatrix {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let group = AnimGroup::create(loader, data)?;
        let last_frame = data.read_u16()?;
        Ok(Self { group, last_frame })
    }
}
