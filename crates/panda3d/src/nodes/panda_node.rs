use hashbrown::HashMap;

use super::bounding_volume::BoundsType;
use super::prelude::*;

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct PandaNode {
    name: String,

    state: Option<u32>,
    transform: Option<u32>,
    effects: Option<u32>,

    draw_control_mask: u32,
    draw_show_mask: u32,
    into_collide_mask: u32,

    bounds_type: BoundsType,

    tag_data: HashMap<String, String>,

    parents: Vec<Option<u32>>,
    children: Vec<(Option<u32>, i32)>,
    stashed: Vec<(Option<u32>, i32)>,
}

impl PandaNode {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        //Main fillin
        let name = data.read_string()?;
        println!("{}", name);

        //CyclerData
        //RenderState*
        let state = loader.read_pointer(data)?;
        //TransformState*
        let transform = loader.read_pointer(data)?;
        //RenderEffects*
        let effects = loader.read_pointer(data)?;

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

        //Since we know the exact number of tags, pre-allocate
        let num_tags = data.read_u32()?;
        let mut tag_data = HashMap::with_capacity(num_tags as usize);

        //Read tags
        for _ in 0..num_tags {
            tag_data.insert(data.read_string()?, data.read_string()?);
        }

        let num_parents = data.read_u16()?;
        let mut parents = Vec::with_capacity(num_parents as usize);
        for _ in 0..num_parents {
            parents.push(loader.read_pointer(data)?);
        }

        let num_children = data.read_u16()?;
        let mut children = Vec::with_capacity(num_children as usize);
        for _ in 0..num_children {
            let pointer = loader.read_pointer(data)?;
            let sort = data.read_i32()?;
            children.push((pointer, sort));
        }

        let num_stashed = data.read_u16()?;
        let mut stashed = Vec::with_capacity(num_stashed as usize);
        for _ in 0..num_stashed {
            let pointer = loader.read_pointer(data)?;
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
