use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct PandaNode {
    pub name: String,

    /// Reference to the associated RenderState
    pub state: u32,
    /// Reference to the associated TransformState
    pub transform: u32,
    /// Reference to the associated RenderEffects
    pub effects: u32,

    pub draw_control_mask: u32,
    pub draw_show_mask: u32,
    pub into_collide_mask: u32,

    pub bounds_type: BoundsType,

    pub tag_data: HashMap<String, String>,

    /// Reference to all parent nodes (may be derived from PandaNode)
    pub parents: Vec<u32>,
    /// Reference to all children nodes (may be derived from PandaNode)
    pub children: Vec<(u32, i32)>,
    pub stashed: Vec<(u32, i32)>,
}

impl PandaNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        // Main fillin
        let name = data.read_string()?;

        // Cycler Data
        let state = loader.read_pointer(data)?.unwrap();
        let transform = loader.read_pointer(data)?.unwrap();
        let effects = loader.read_pointer(data)?.unwrap();

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
                draw_show_mask = draw_mask
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

        // These are processed as fillin_up_list/fillin_down_list
        let num_parents = data.read_u16()?;
        let mut parents = Vec::with_capacity(num_parents as usize);
        for _ in 0..num_parents {
            parents.push(loader.read_pointer(data)?.unwrap());
        }
        //TODO: sort parent nodes? They're based on pointer order so they're different per session

        let num_children = data.read_u16()?;
        let mut children = Vec::with_capacity(num_children as usize);
        for _ in 0..num_children {
            let pointer = loader.read_pointer(data)?.unwrap();
            let sort = data.read_i32()?;
            children.push((pointer, sort));
        }

        let num_stashed = data.read_u16()?;
        let mut stashed = Vec::with_capacity(num_stashed as usize);
        for _ in 0..num_stashed {
            let pointer = loader.read_pointer(data)?.unwrap();
            let sort = data.read_i32()?;
            stashed.push((pointer, sort));
        }

        Ok(PandaNode {
            name,
            state,
            transform,
            effects,
            draw_control_mask,
            draw_show_mask,
            into_collide_mask,
            bounds_type,
            tag_data,
            parents,
            children,
            stashed,
        })
    }
}
