use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct PandaNode {
    pub name: String,

    /// Reference to the associated RenderState
    pub state_ref: u32,
    /// Reference to the associated TransformState
    pub transform_ref: u32,
    /// Reference to the associated RenderEffects
    pub effects_ref: u32,

    pub draw_control_mask: u32,
    pub draw_show_mask: u32,
    pub into_collide_mask: u32,

    pub bounds_type: BoundsType,

    pub tag_data: HashMap<String, String>,

    /// Reference to all parent nodes (may be derived from PandaNode)
    pub parent_refs: Vec<u32>,
    /// Reference to all children nodes (may be derived from PandaNode)
    pub child_refs: Vec<(u32, i32)>,
    pub stashed_refs: Vec<(u32, i32)>,
}

impl Node for PandaNode {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        // Main fillin
        let name = data.read_string()?;

        // Cycler Data
        let state_ref = loader.read_pointer(data)?.unwrap();
        let transform_ref = loader.read_pointer(data)?.unwrap();
        let effects_ref = loader.read_pointer(data)?.unwrap();

        let draw_control_mask: u32;
        let draw_show_mask: u32;
        if loader.get_minor_version() >= 2 {
            draw_control_mask = data.read_u32()?;
            draw_show_mask = data.read_u32()?;
        } else {
            // Older nodes just stored it as a single value
            let mut draw_mask = data.read_u32()?;

            if draw_mask == 0 {
                //Hidden node
                draw_control_mask = 1 << 31;
                draw_show_mask = !(1 << 31);
            } else if draw_mask == !0 {
                //Visible node
                draw_control_mask = 0;
                draw_show_mask = !0;
            } else {
                draw_mask &= !(1 << 31);
                draw_control_mask = !draw_mask;
                draw_show_mask = draw_mask;
            }
        }

        let into_collide_mask = data.read_u32()?;

        let bounds_type = match loader.get_minor_version() >= 19 {
            true => BoundsType::from(data.read_u8()?),
            false => BoundsType::Default,
        };

        let num_tags = data.read_u32()?;
        let mut tag_data = HashMap::with_capacity(num_tags as usize);
        for _ in 0..num_tags {
            tag_data.insert(data.read_string()?, data.read_string()?);
        }

        // These are processed as fillin_up_list/fillin_down_list/fillin_down_list
        let num_parents = data.read_u16()?;
        let mut parent_refs = Vec::with_capacity(num_parents as usize);
        for _ in 0..num_parents {
            parent_refs.push(loader.read_pointer(data)?.unwrap());
        }
        //TODO: sort parent nodes? They're based on pointer order so they're different per session

        let num_children = data.read_u16()?;
        let mut child_refs = Vec::with_capacity(num_children as usize);
        for _ in 0..num_children {
            let pointer = loader.read_pointer(data)?.unwrap();
            let sort = data.read_i32()?;
            child_refs.push((pointer, sort));
        }

        let num_stashed = data.read_u16()?;
        let mut stashed_refs = Vec::with_capacity(num_stashed as usize);
        for _ in 0..num_stashed {
            let pointer = loader.read_pointer(data)?.unwrap();
            let sort = data.read_i32()?;
            stashed_refs.push((pointer, sort));
        }

        Ok(PandaNode {
            name,
            state_ref,
            transform_ref,
            effects_ref,
            draw_control_mask,
            draw_show_mask,
            into_collide_mask,
            bounds_type,
            tag_data,
            parent_refs,
            child_refs,
            stashed_refs,
        })
    }
}

impl GraphDisplay for PandaNode {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{PandaNode|")?;
        }

        // Fields
        write!(label, "name: {}|", self.name)?;
        connections.push(self.state_ref);
        connections.push(self.transform_ref);
        connections.push(self.effects_ref);
        write!(label, "draw_control_mask: {:#010X}|", self.draw_control_mask)?;
        write!(label, "draw_show_mask: {:#010X}|", self.draw_show_mask)?;
        write!(label, "into_collide_mask: {:#010X}|", self.into_collide_mask)?;
        write!(label, "bounds_type: {:#?}", self.bounds_type)?;

        if !self.tag_data.is_empty() {
            write!(label, "|{{tag_data|")?;
            let mut first = true;
            for (key, value) in &self.tag_data {
                if !first {
                    write!(label, "|")?;
                }
                write!(label, "{}: {}", key, value)?;
                first = false;
            }
            write!(label, "}}")?;
        }
        // Ignore parents, since we should already have made that
        for child_ref in &self.child_refs {
            connections.push(child_ref.0);
        }
        for stashed_ref in &self.stashed_refs {
            connections.push(stashed_ref.0);
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
