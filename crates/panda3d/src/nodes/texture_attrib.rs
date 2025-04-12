use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct StageNode {
    pub sampler: Option<SamplerState>,
    /// Reference to the associated TextureStage data
    pub texture_stage_ref: u32,
    /// Reference to the associated Texture data
    pub texture_ref: u32,
    pub priority: i32,
    pub implicit_sort: u16,
}

impl StageNode {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
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

        let sampler = match loader.get_minor_version() >= 36 {
            true => {
                match data.read_bool()? {
                    true => Some(SamplerState::create(loader, data)?),
                    false => None,
                }
            }
            false => None,
        };

        Ok(Self { sampler, texture_stage_ref, texture_ref, priority, implicit_sort })
    }
}

impl GraphDisplay for StageNode {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{StageNode")?;
        }

        // Fields
        if let Some(sampler) = &self.sampler {
            if is_root {
                write!(label, "|")?;
            }
            sampler.write_data(label, connections, false)?;
        }
        connections.push(self.texture_stage_ref);
        connections.push(self.texture_ref);
        if is_root {
            write!(label, "|")?;
        }
        write!(label, "priority: {}", self.priority)?;
        write!(label, "|implicit_sort: {:#06X}", self.implicit_sort)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct TextureAttrib {
    pub off_all_stages: bool,
    /// References to associated TextureStage data
    pub off_stage_refs: Vec<u32>,
    pub on_stages: Vec<StageNode>,
}

impl Node for TextureAttrib {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
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

impl GraphDisplay for TextureAttrib {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{TextureAttrib|")?;
        }

        // Fields
        write!(label, "off_all_stages: {}", self.off_all_stages)?;
        for reference in &self.off_stage_refs {
            connections.push(*reference);
        }
        for stage in &self.on_stages {
            write!(label, "|")?;
            stage.write_data(label, connections, false)?;
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
