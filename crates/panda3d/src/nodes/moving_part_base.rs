use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct MovingPartBase {
    pub group: PartGroup,
    pub forced_channel_ref: Option<u32>,
}

impl MovingPartBase {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let group = PartGroup::create(loader, data)?;
        let forced_channel_ref = match loader.get_minor_version() >= 20 {
            true => loader.read_pointer(data)?,
            false => None,
        };

        Ok(Self { group, forced_channel_ref })
    }
}
