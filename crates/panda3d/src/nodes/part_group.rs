use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct PartGroup {
    pub name: String,
    pub child_refs: Vec<u32>,
}

impl Node for PartGroup {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;

        if loader.get_minor_version() == 11 {
            unimplemented!("I don't have a BAM file this old - contact me");
        }

        let num_children = data.read_u16()?;
        let mut child_refs = Vec::with_capacity(num_children as usize);
        for _ in 0..num_children {
            child_refs.push(loader.read_pointer(data)?.unwrap());
        }

        Ok(Self { name, child_refs })
    }
}

impl GraphDisplay for PartGroup {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{PartGroup|")?;
        }

        // Fields
        let name = self.name.replace('<', "\\<").replace('>', "\\>");
        // This is a hack because PartGroup often has <skeleton> which graphviz doesn't like
        write!(label, "name: {}", name)?;
        for reference in &self.child_refs {
            connections.push(*reference);
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
