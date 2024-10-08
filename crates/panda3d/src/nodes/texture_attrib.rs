use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct StageNode {
    pub sampler: Option<SamplerState>,
    /// Reference to the associated TextureStage data
    pub texture_stage_ref: u32,
    /// Reference to the associated Texture data
    pub texture_ref: u32,
    pub priority: i32,
    pub implicit_sort: u16,
    pub texcoord_index: i32,
}

impl StageNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let texture_stage_ref = loader.read_pointer(data)?.unwrap();
        let texture_ref = loader.read_pointer(data)?.unwrap();

        let implicit_sort = match loader.get_minor_version() >= 15 {
            true => data.read_u16()?,
            false => 0,
        };

        let priority = match loader.get_minor_version() >= 23 {
            true => data.read_i32()?,
            false => 0,
        };

        let sampler: Option<SamplerState>;
        if loader.get_minor_version() >= 36 {
            sampler = match data.read_bool()? {
                true => Some(SamplerState::create(loader, data)?),
                false => None,
            };
        } else {
            sampler = None;
        }

        Ok(Self {
            sampler,
            texture_stage_ref,
            texture_ref,
            priority,
            implicit_sort,
            texcoord_index: 0,
        })
    }
}

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct TextureAttrib {
    pub off_all_stages: bool,
    /// References to associated TextureStage data
    pub off_stage_refs: Vec<u32>,
    pub on_stages: Vec<StageNode>,
}

impl TextureAttrib {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let off_all_stages = data.read_bool()?;

        let num_off_stages = data.read_u16()?;
        let mut off_stage_refs = Vec::with_capacity(num_off_stages as usize);
        for _ in 0..num_off_stages {
            let texture_stage_ref = loader.read_pointer(data)?.unwrap();
            off_stage_refs.push(texture_stage_ref);
        }

        let num_on_stages = data.read_u16()?;
        let mut on_stages = Vec::with_capacity(num_on_stages as usize);
        let mut next_implicit_sort = 0;
        for n in 0..num_on_stages {
            let mut stage_node = StageNode::create(loader, data)?;
            // Before 6.15, we didn't store the implicit order, so use the stored order
            if loader.get_minor_version() < 15 {
                stage_node.implicit_sort = n;
            }

            // Now we need to calculate the actual order, since it can be different than above
            next_implicit_sort = core::cmp::max(next_implicit_sort, stage_node.implicit_sort + 1);
            stage_node.implicit_sort = next_implicit_sort;
            next_implicit_sort += 1;

            on_stages.push(stage_node);
        }

        Ok(Self { off_all_stages, off_stage_refs, on_stages })
    }
}
