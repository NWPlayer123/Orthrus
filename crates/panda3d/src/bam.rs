//! Adds support for the Binary Asset format used by the Panda3D engine.
//!
//! # Overview
//! There does not seem to be much documentation on the origins of this file format in the Panda3D codebase,
//! so this module attempts to give an overview.
//!
//! This format was designed to store any amount of Panda3D objects (all of which are derived from
//! TypedWritable), and is most often used to store models and/or animations, hence the most common file
//! extension being ".bam", which stands for Binary Animation and Models. There is also ".boo", which stands
//! for Binary Other Objects.
//!
//! It is used to directly represent Panda3D's internal scene graph hierarchy in a binary format, as compared
//! to .egg which is meant to be a greatly simplified human-readable version that can be edited by other
//! people or programs as well as being an "intermediate" format between more typical model formats.
//!
//! # Revisions

#[cfg(feature = "std")]
use std::{io::prelude::*, path::Path};

use hashbrown::HashMap;
use num_enum::FromPrimitive;
use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::{
    common::*,
    nodes::{
        dispatch::{NodeStorage, StoredType},
        prelude::*,
    },
};

/// Error conditions for when working with Multifile archives.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum Error {
    #[snafu(display("Formatting Error {source}"))]
    FormatError { source: core::fmt::Error },

    /// Thrown if a [`std::io::Error`] happened when trying to read/write files.
    #[snafu(display("Filesystem Error {source}"))]
    FileError { source: std::io::Error },

    /// Thrown if trying to read the file out of its current bounds.
    #[snafu(display("Reached the end of the current stream!"))]
    EndOfFile,

    /// Thrown if UTF-8 validation fails when trying to convert a string.
    #[snafu(display("{source}"))]
    InvalidString { source: Utf8ErrorSource },

    /// Thrown if the header contains a magic number other than "pbj\0\n\r".
    #[snafu(display("Invalid Magic! Expected {expected:?}."))]
    InvalidMagic { expected: &'static [u8] },

    /// Thrown if the header version is too new to be supported.
    #[snafu(display("Invalid Version! Expected <= v{}.", BinaryAsset::CURRENT_VERSION))]
    InvalidVersion,

    /// Thrown if unable to downcast to a specific type.
    #[snafu(display("Node is not of type {type_name}"))]
    InvalidType { type_name: &'static str },
}

// Implemented so that the GraphViz implementations can easily coerce to this error
impl From<core::fmt::Error> for Error {
    #[inline]
    fn from(source: core::fmt::Error) -> Self {
        Self::FormatError { source }
    }
}

// Implemented since read_/write_ are so integral to the program
impl From<DataError> for Error {
    #[inline]
    fn from(error: DataError) -> Self {
        match error {
            DataError::EndOfFile => Error::EndOfFile,
            DataError::Io { source } => Error::FileError { source },
            DataError::InvalidString { source } => Error::InvalidString { source },
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Header {
    pub(crate) version: Version,
    endian: Endian,
    /// BAM files starting with 6.27 support reading either floats or doubles (false/true)
    pub(crate) use_double: bool,
}

impl Header {
    #[inline]
    fn create(mut data: Datagram) -> Result<Self, self::Error> {
        let version = Version { major: data.read_u16()?, minor: data.read_u16()? };
        let endian = match data.read_u8()? {
            0 => Endian::Big,
            1 => Endian::Little,
            _ => Endian::default(),
        };
        let use_double = match version.minor >= 27 {
            true => data.read_u8()? != 0,
            false => false,
        };
        Ok(Self { version, endian, use_double })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum ObjectCode {
    /// Includes an object definition, always paired with a Pop.
    Push,
    /// Paired with a Push in order to allow for nesting.
    Pop,
    /// Includes an object definition, does not change nesting level.
    #[default]
    Adjunct,
    /// List of object IDs that were deallocated by the sender, ???
    Remove,
    /// Additional file data that can be referenced by other objects, ???
    FileData,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ObjectsLeft {
    /// Prior to BAM 6.21
    ObjectCount { num_extra_objects: i32 },
    /// Starting with BAM 6.21
    NestingLevel { nesting_level: i32 },
}

impl Default for ObjectsLeft {
    fn default() -> Self {
        Self::NestingLevel { nesting_level: 0 }
    }
}

#[derive(Debug, Default)]
pub struct BinaryAsset {
    /// Holds all BAM metadata needed for parsing
    pub(crate) header: Header,
    /// Used to store whether the BAM stream has more objects to read
    pub(crate) objects_left: ObjectsLeft,
    /// Used if there are more than 65535 Object IDs
    pub(crate) long_object_id: bool,
    /// Used if there are more than 65535 Pointer to Array IDs
    pub(crate) long_pta_id: bool,
    pub(crate) type_registry: HashMap<u16, String>,
    pub nodes: NodeStorage,
    pub(crate) arrays: Vec<Vec<u32>>,
}

impl BinaryAsset {
    /// Latest revision of the BAM format. For more info, see [here](self#revisions).
    pub const CURRENT_VERSION: Version = Version { major: 6, minor: 45 };
    /// Unique identifier that tells us if we're reading a Panda3D Binary Object.
    pub const MAGIC: &'static [u8] = b"pbj\0\n\r";
    /// Earliest supported revision of the BAM format. For more info, see [here](self#revisions).
    pub const MINIMUM_VERSION: Version = Version { major: 6, minor: 14 };

    #[must_use]
    #[inline]
    pub fn get_minor_version(&self) -> u16 {
        self.header.version.minor
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(input: P) -> Result<Self, self::Error> {
        let data = std::fs::read(input).context(FileSnafu)?;
        Self::load(data)
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<Self, self::Error> {
        fn inner(input: Box<[u8]>) -> Result<BinaryAsset, self::Error> {
            let mut data = DataCursor::new(input, Endian::Little);

            // Read the magic and make sure we're actually parsing a BAM file
            const LENGTH: usize = BinaryAsset::MAGIC.len();
            ensure!(data.len()? >= LENGTH as u64, EndOfFileSnafu);
            let magic = data.read_slice(LENGTH)?;
            ensure!(
                magic == BinaryAsset::MAGIC,
                InvalidMagicSnafu { expected: BinaryAsset::MAGIC }
            );

            // The first datagram is always the header data
            let header = Header::create(Datagram::new(&mut data, Endian::Little, false)?)?;
            ensure!(
                header.version.major == BinaryAsset::CURRENT_VERSION.major
                    && header.version.minor >= BinaryAsset::MINIMUM_VERSION.minor
                    && header.version.minor <= BinaryAsset::CURRENT_VERSION.minor,
                InvalidVersionSnafu
            );

            // Create the BinaryAsset instance so we can start constructing all the objects
            let objects_left = match header.version.minor >= 21 {
                true => ObjectsLeft::NestingLevel { nesting_level: 0 },
                false => ObjectsLeft::ObjectCount { num_extra_objects: 0 },
            };
            let mut bamfile = BinaryAsset { header, objects_left, ..Default::default() };

            bamfile.load_inner(data)?;

            Ok(bamfile)
        }
        inner(input.into())
    }

    #[inline]
    fn load_inner(&mut self, mut data: DataCursor) -> Result<(), self::Error> {
        // Read the initial object
        let mut datagram = Datagram::new(&mut data, self.header.endian, self.header.use_double)?;
        self.read_object(&mut datagram)?;

        loop {
            //println!("Reading datagram at {:X}", data.position()?);
            match self.objects_left {
                ObjectsLeft::ObjectCount { mut num_extra_objects } => {
                    if num_extra_objects > 0 {
                        datagram = Datagram::new(&mut data, self.header.endian, self.header.use_double)?;
                        self.read_object(&mut datagram)?;
                        num_extra_objects -= 1;
                        self.objects_left = ObjectsLeft::ObjectCount { num_extra_objects }
                    } else {
                        break;
                    }
                }
                ObjectsLeft::NestingLevel { nesting_level } => {
                    if nesting_level > 0 {
                        datagram = Datagram::new(&mut data, self.header.endian, self.header.use_double)?;
                        self.read_object(&mut datagram)?;
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn read_object(&mut self, data: &mut Datagram) -> Result<(), self::Error> {
        // If we're reading a file 6.21 or newer, control flow codes are in the data stream, so
        // match against the enum variant
        if let ObjectsLeft::NestingLevel { ref mut nesting_level } = self.objects_left {
            let object_code = ObjectCode::from(data.read_u8()?);
            match object_code {
                ObjectCode::Push => {
                    *nesting_level += 1;
                }
                ObjectCode::Pop => {
                    *nesting_level -= 1;
                    return Ok(());
                }
                ObjectCode::Adjunct => {}
                _ => {
                    todo!("Remove and FileData are unimplemented, need a test case, pls message me.")
                }
            }
        }

        // Check the type handle, see if we need to register any new types
        let type_handle = self.read_handle(data)?;
        // Read the Object ID and process it
        let _object_id = self.read_object_id(data)? - 1;
        //println!("Object ID {}", _object_id);
        /*println!(
            "Initial type data {:#X}, Data size {:#X}\n",
            data.position()?,
            data.len()?
        );*/

        if type_handle != 0 {
            // Now we need to read the data of the associated type using the "fillin" functions
            // For now I'm combining them into a single function
            let type_name = self.type_registry.get_mut(&type_handle).expect("a").to_owned();
            //println!("Filling in {} from {:#X}", type_name, data.position()?);
            self.fillin(data, &type_name)?;
        }
        if data.position()? != data.len()? {
            println!(
                "Finished at {:#X}, Data size {:#X}\n",
                data.position()?,
                data.len()?
            );
        }
        Ok(())
    }

    fn read_handle(&mut self, data: &mut Datagram) -> Result<u16, self::Error> {
        let type_handle = data.read_u16()?;

        // Found a new type, read its string and register it
        if !self.type_registry.contains_key(&type_handle) {
            let type_name = data.read_string()?;
            //println!("Registering Type {type_name} -> {type_handle}");
            self.type_registry.insert(type_handle, type_name);

            //Check for any parent classes we need to register
            let parent_count = data.read_u8()?;
            //println!("Parent Count: {parent_count}");
            for _ in 0..parent_count {
                self.read_handle(data)?;
            }
        }

        Ok(type_handle)
    }

    fn read_object_id(&mut self, data: &mut Datagram) -> Result<u32, self::Error> {
        let object_id;
        if self.long_object_id {
            object_id = data.read_u32()?;
        } else {
            object_id = data.read_u16()?.into();
            if object_id == 0xFFFF {
                self.long_object_id = true;
            }
        }
        Ok(object_id)
    }

    pub(crate) fn read_pta_id(&mut self, data: &mut Datagram) -> Result<u32, self::Error> {
        let pta_id;
        if self.long_pta_id {
            pta_id = data.read_u32()?;
        } else {
            pta_id = data.read_u16()?.into();
            if pta_id == 0xFFFF {
                self.long_pta_id = true;
            }
        }
        Ok(pta_id)
    }

    pub(crate) fn read_pointer(&mut self, data: &mut Datagram) -> Result<Option<u32>, self::Error> {
        let object_id = self.read_object_id(data)?;
        //println!("Object ID ptrto {}", object_id);
        if object_id != 0 {
            // objects_left will only be ObjectCount on pre-6.21 so this should be safe
            if let ObjectsLeft::ObjectCount { ref mut num_extra_objects } = self.objects_left {
                *num_extra_objects -= 1;
            }
            // ObjectID 0 is reserved, but we still want to store indices as zero-indexed, so subtract 1
            return Ok(Some(object_id - 1));
        }
        Ok(None)
    }

    //should really be using make_from_bam as an entrypoint
    fn fillin(&mut self, data: &mut Datagram<'_>, type_name: &str) -> Result<(), self::Error> {
        //println!("{type_name}");
        match type_name {
            "AnimBundle" => self.create_node::<AnimBundle>(data),
            "AnimBundleNode" => self.create_node::<AnimBundleNode>(data),
            "AnimChannelMatrixXfmTable" => self.create_node::<AnimChannelMatrixXfmTable>(data),
            "AnimGroup" => self.create_node::<AnimGroup>(data),
            "BillboardEffect" => self.create_node::<BillboardEffect>(data),
            "Character" => self.create_node::<Character>(data),
            "CharacterJoint" => self.create_node::<CharacterJoint>(data),
            "CharacterJointBundle" => self.create_node::<PartBundle>(data),
            "CharacterJointEffect" => self.create_node::<CharacterJointEffect>(data),
            "CollisionCapsule" => self.create_node::<CollisionCapsule>(data),
            "CollisionNode" => self.create_node::<CollisionNode>(data),
            "CollisionPolygon" => self.create_node::<CollisionPolygon>(data),
            "CollisionSphere" => self.create_node::<CollisionSphere>(data),
            "CollisionTube" => self.create_node::<CollisionCapsule>(data),
            "ColorAttrib" => self.create_node::<ColorAttrib>(data),
            "CullBinAttrib" => self.create_node::<CullBinAttrib>(data),
            "CullFaceAttrib" => self.create_node::<CullFaceAttrib>(data),
            "DecalEffect" => self.create_node::<DecalEffect>(data),
            "DepthWriteAttrib" => self.create_node::<DepthWriteAttrib>(data),
            "Geom" => self.create_node::<Geom>(data),
            "GeomNode" => self.create_node::<GeomNode>(data),
            "GeomTriangles" => self.create_node::<GeomPrimitive>(data),
            "GeomTristrips" => self.create_node::<GeomPrimitive>(data),
            "GeomVertexArrayData" => self.create_node::<GeomVertexArrayData>(data),
            "GeomVertexArrayFormat" => self.create_node::<GeomVertexArrayFormat>(data),
            "GeomVertexData" => self.create_node::<GeomVertexData>(data),
            "GeomVertexFormat" => self.create_node::<GeomVertexFormat>(data),
            "InternalName" => self.create_node::<InternalName>(data),
            "JointVertexTransform" => self.create_node::<JointVertexTransform>(data),
            "LODNode" => self.create_node::<LODNode>(data),
            "ModelNode" => self.create_node::<ModelNode>(data),
            "ModelRoot" => self.create_node::<ModelNode>(data),
            "PandaNode" => self.create_node::<PandaNode>(data),
            "PartGroup" => self.create_node::<PartGroup>(data),
            "RenderEffects" => self.create_node::<RenderEffects>(data),
            "RenderState" => self.create_node::<RenderState>(data),
            "Texture" => self.create_node::<Texture>(data),
            "TextureAttrib" => self.create_node::<TextureAttrib>(data),
            "TextureStage" => self.create_node::<TextureStage>(data),
            "TransformBlendTable" => self.create_node::<TransformBlendTable>(data),
            "TransformState" => self.create_node::<TransformState>(data),
            "TransparencyAttrib" => self.create_node::<TransparencyAttrib>(data),
            "UserVertexTransform" => self.create_node::<UserVertexTransform>(data),
            _ => todo!("{type_name}"),
        }
    }

    fn create_node<T: Node + StoredType>(&mut self, data: &mut Datagram) -> Result<(), Error> {
        let node = T::create(self, data)?;
        //println!("{:#?}", node);
        self.nodes.push(node);
        Ok(())
    }
}

#[cfg(feature = "std")]
pub struct GraphWriter {
    file: std::fs::File,
}

impl GraphWriter {
    fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = std::fs::File::create(&path)?;

        let graph_name = path.as_ref().file_stem().and_then(|s| s.to_str()).unwrap_or("graph");

        writeln!(file, "digraph \"{}\" {{", graph_name)?;
        //writeln!(file, "    graph [rankdir=LR]")?;
        writeln!(
            file,
            "    node [shape=record, style=rounded, fontname=\"Consolas\", fontsize=20]"
        )?;
        writeln!(file)?;

        Ok(GraphWriter { file })
    }

    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        writeln!(self.file, "    {}", line)
    }

    fn write_edge(&mut self, from: &str, to: &str, label: Option<&str>) -> std::io::Result<()> {
        match label {
            Some(l) => self.write_line(&format!("{} -> {} [label=\"{}\"];", from, to, l)),
            None => self.write_line(&format!("{} -> {};", from, to)),
        }
    }

    fn write_node(&mut self, name: &str, label: Option<&str>) -> std::io::Result<()> {
        match label {
            Some(l) => self.write_line(&format!("{} [label=\"{}\"];", name, l)),
            None => self.write_line(&format!("{};", name)),
        }
    }

    fn close(mut self) -> std::io::Result<()> {
        writeln!(self.file, "}}")
    }

    pub fn write_nodes<P: AsRef<Path>>(nodes: &NodeStorage, path: P) -> Result<(), Error> {
        let mut graph_writer = Self::new(path).context(FileSnafu)?;

        for n in 0..nodes.len() {
            let node = nodes.get(n).unwrap();
            let mut label = String::new();
            let mut connections = Vec::new();
            node.write_graph_data(&mut label, &mut connections)?;
            let name = format!("node_{}", n);
            graph_writer.write_node(&name, Some(&label)).context(FileSnafu)?;
            for connection in connections {
                let to = format!("node_{}", connection);
                graph_writer.write_edge(&name, &to, None).context(FileSnafu)?;
            }
        }

        graph_writer.close().context(FileSnafu)?;
        Ok(())
    }
}

// TODO: stuff I can already see, it would be nice to add labels to connections (&mut Vec<(u32, &'static
// str)>), and it would be nice to have read access to NodeStorage so we can get std::any::type_name() for
// NodePath
#[cfg(feature = "std")]
pub trait GraphDisplay {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), Error>;
}
