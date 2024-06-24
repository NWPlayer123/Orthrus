use super::geom_enums::*;
use super::prelude::*;

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct GeomPrimitive {
    shade_model: ShadeModel,
    first_vertex: i32,
    num_vertices: i32,
    index_type: NumericType,
    usage_hint: UsageHint,
    vertices: Option<u32>,
    ptr_to_array: u32,
    array: Vec<u32>,
}

impl GeomPrimitive {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        //cycler data
        let shade_model = ShadeModel::from(data.read_u8()?);
        let first_vertex = data.read_i32()?;
        let num_vertices = data.read_i32()?;
        let index_type = NumericType::from(data.read_u8()?);
        let usage_hint = UsageHint::from(data.read_u8()?);
        let vertices = loader.read_pointer(data)?;
        let ptr_to_array = loader.read_pta_id(data)?;
        let mut array = Vec::new();
        //TODO: clean this up after refactoring BinaryAsset
        if ptr_to_array == 0 {
            unimplemented!("Not sure what to do with this");
        } else {
            println!("{}", data.position());
            let size = data.read_u32()?;
            for _ in 0..size {
                array.push(data.read_u32()?);
            }
        }

        Ok(Self {
            shade_model,
            first_vertex,
            num_vertices,
            index_type,
            usage_hint,
            vertices,
            ptr_to_array,
            array,
        })
    }
}
