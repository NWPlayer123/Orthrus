use super::prelude::*;
use super::sampler_state::SamplerState;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct StageNode {
    sampler: Option<SamplerState>,
    texture_stage: Option<u32>,
    texture: Option<u32>,
    r#override: i32,
    pub(crate) implicit_sort: u16,
    texcoord_index: i32,
}

impl StageNode {
    pub fn create(
        loader: &mut BinaryAsset,
        data: &mut Datagram,
        mut implicit_sort: u16,
    ) -> Result<Self, bam::Error> {
        //StageNode*
        let texture_stage = loader.read_pointer(data)?;
        //Texture*
        let texture = loader.read_pointer(data)?;

        if loader.get_minor_version() >= 15 {
            implicit_sort = data.read_u16()?;
        }

        let r#override = match loader.get_minor_version() >= 23 {
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
            texture_stage,
            texture,
            r#override,
            implicit_sort,
            texcoord_index: 0,
        })
    }
}

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct TextureAttrib {
    off_all_stages: bool,
    off_stages: Vec<Option<u32>>,
    on_stages: Vec<StageNode>,
}

impl TextureAttrib {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let off_all_stages = data.read_bool()?;

        let num_off_stages = data.read_u16()?;
        let mut off_stages = Vec::with_capacity(num_off_stages as usize);
        for _ in 0..num_off_stages {
            //StageNode*
            let texture_stage = loader.read_pointer(data)?;
            off_stages.push(texture_stage);
        }

        let num_on_stages = data.read_u16()?;
        let mut on_stages = Vec::with_capacity(num_on_stages as usize);
        let mut next_implicit_sort = 0;
        for n in 0..num_on_stages {
            let mut stage_node = StageNode::create(loader, data, n)?;

            next_implicit_sort = core::cmp::max(next_implicit_sort, stage_node.implicit_sort + 1);
            stage_node.implicit_sort = next_implicit_sort;
            next_implicit_sort += 1;

            on_stages.push(stage_node);
        }

        Ok(Self {
            off_all_stages,
            off_stages,
            on_stages,
        })
    }
}
