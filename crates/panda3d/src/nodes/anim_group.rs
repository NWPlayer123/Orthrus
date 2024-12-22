use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimGroup {
    pub name: String,
    pub root_ref: u32,
    pub child_refs: Vec<u32>,
}

impl Node for AnimGroup {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;
        let root_ref = loader.read_pointer(data)?.unwrap();
        let num_children = data.read_u16()?;
        let mut child_refs = Vec::with_capacity(num_children as usize);
        for _ in 0..num_children {
            let child_ref = loader.read_pointer(data)?.unwrap();
            child_refs.push(child_ref);
        }
        Ok(Self { name, root_ref, child_refs })
    }
}

impl GraphDisplay for AnimGroup {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{AnimGroup|")?;
        }

        // Fields
        let name = self.name.replace('<', "\\<").replace('>', "\\>");
        // This is a hack because PartGroup often has <skeleton> which graphviz doesn't like
        write!(label, "name: {}", name)?;
        // root_ref just makes cyclic references so eh
        for child in &self.child_refs {
            connections.push(*child);
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
