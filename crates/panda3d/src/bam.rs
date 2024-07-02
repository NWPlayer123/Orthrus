//! Adds support for the Binary Asset format used by the Panda3D engine.
//!
//! # Overview
//! There does not seem to be much documentation of the origins of this file format in the Panda3D
//! codebase, but here is the general outline.
//!
//! This format is a generic one that can store any amount of Panda3D objects (all of which are
//! derived from TypedWritable), and is most often used to store models and/or animations, hence the
//! most common file extension being ".bam", which stands for Binary Animation and Models. There is
//! also ".boo", which stands for Binary Other Objects.
//!
//! It is used to represent Panda3D's internal scene graph hierarchy in a binary file format, as
//! compared to .egg which is meant to be human-readable and editable by other programs.
//!
//! # Revisions

#[cfg(feature = "std")]
use std::path::Path;

use hashbrown::HashMap;
use num_enum::FromPrimitive;
use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::common::*;
use crate::nodes::prelude::*;

/// Error conditions for when working with Multifile archives.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    /// Thrown when trying to open a file or folder that doesn't exist.
    #[snafu(display("Unable to find file/folder!"))]
    NotFound,
    /// Thrown if reading/writing tries to go out of bounds.
    #[snafu(display("Unexpected End-Of-File!"))]
    EndOfFile,
    /// Thrown when unable to open a file or folder.
    #[snafu(display("No permissions to open file/folder!"))]
    PermissionDenied,
    /// Thrown if the header contains a magic number other than "pbj\0\n\r".
    #[snafu(display("Invalid Magic! Expected {:?}.", BinaryAsset::MAGIC))]
    InvalidMagic,
    /// Thrown if the header version is too new to be supported.
    #[snafu(display("Invalid BAM Version! Expected <= v{}.", BinaryAsset::CURRENT_VERSION))]
    InvalidVersion,
    /// Thrown if the header has an unknown endianness.
    #[snafu(display("Invalid File Endian! Malformed BAM file?"))]
    InvalidEndian,
    /// Thrown if the header has an unknown endianness.
    #[snafu(display("Invalid Node Type! Malformed BAM file?"))]
    InvalidType,
    /// Thrown if trying to convert an enum but the value is outside of the bounds
    #[snafu(display("Invalid Enum Variant! Malformed BAM file?"))]
    InvalidEnum,
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::UnexpectedEof => Self::EndOfFile,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            kind => {
                panic!("Unexpected std::io::error: {kind}! Something has gone horribly wrong")
            }
        }
    }
}

impl From<data::Error> for Error {
    #[inline]
    fn from(error: data::Error) -> Self {
        match error {
            data::Error::EndOfFile => Self::EndOfFile,
            _ => panic!("Unexpected data::error! Something has gone horribly wrong"),
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
    fn create(data: &mut Datagram) -> Result<Self, self::Error> {
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
    pub(crate) nodes: Vec<PandaObject>,
    pub(crate) arrays: Vec<Vec<u32>>,
}

impl BinaryAsset {
    /// Latest revision of the BAM format. For more info, see [here](self#revisions).
    pub const CURRENT_VERSION: Version = Version { major: 6, minor: 45 };
    /// Unique identifier that tells us if we're reading a Panda3D Binary Object.
    pub const MAGIC: [u8; 6] = *b"pbj\0\n\r";
    /// Earliest supported revision of the BAM format. For more info, see [here](self#revisions).
    pub const MINIMUM_VERSION: Version = Version { major: 6, minor: 14 };

    pub fn get_minor_version(&self) -> u16 {
        self.header.version.minor
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(input: P) -> Result<Self, self::Error> {
        let data = std::fs::read(input)?;
        Self::load(data)
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<Self, self::Error> {
        let mut data = DataCursor::new(input, Endian::Little);

        // Read the magic and make sure we're actually parsing a BAM file
        let mut magic = [0u8; 6];
        data.read_length(&mut magic)?;
        ensure!(magic == Self::MAGIC, InvalidMagicSnafu);

        // The first datagram is always the header data
        let mut datagram = Datagram::new(&mut data, Endian::Little, false)?;
        let header = Header::create(&mut datagram)?;
        ensure!(
            header.version.major == Self::CURRENT_VERSION.major
                && header.version.minor >= Self::MINIMUM_VERSION.minor
                && header.version.minor <= Self::CURRENT_VERSION.minor,
            InvalidVersionSnafu
        );

        // Create the BinaryAsset instance so we can start constructing all the objects
        let objects_left = match header.version.minor >= 21 {
            true => ObjectsLeft::NestingLevel { nesting_level: 0 },
            false => ObjectsLeft::ObjectCount { num_extra_objects: 0 },
        };
        let mut bamfile = Self {
            header,
            type_registry: HashMap::new(),
            objects_left,
            nodes: vec![PandaObject::Null],
            ..Default::default()
        };

        // Read the initial object
        datagram = Datagram::new(&mut data, bamfile.header.endian, bamfile.header.use_double)?;
        bamfile.read_object(&mut datagram)?;

        loop {
            match bamfile.objects_left {
                ObjectsLeft::ObjectCount { mut num_extra_objects } => {
                    if num_extra_objects > 0 {
                        datagram =
                            Datagram::new(&mut data, bamfile.header.endian, bamfile.header.use_double)?;
                        bamfile.read_object(&mut datagram)?;
                        num_extra_objects -= 1;
                        bamfile.objects_left = ObjectsLeft::ObjectCount { num_extra_objects }
                    } else {
                        break;
                    }
                }
                ObjectsLeft::NestingLevel { nesting_level } => {
                    if nesting_level > 0 {
                        datagram =
                            Datagram::new(&mut data, bamfile.header.endian, bamfile.header.use_double)?;
                        bamfile.read_object(&mut datagram)?;
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(bamfile)
    }

    fn read_object(&mut self, data: &mut Datagram) -> Result<(), self::Error> {
        // If we're reading a file 6.21 or newer, control flow codes are in the data stream, so
        // match against the enum variant
        match self.objects_left {
            ObjectsLeft::NestingLevel { ref mut nesting_level } => {
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
            _ => (),
        }

        // Check the type handle, see if we need to register any new types
        let type_handle = self.read_handle(data)?;
        // Read the Object ID and process it
        let _object_id = self.read_object_id(data)?;
        //println!("Object ID {}", object_id);
        /*println!(
            "Initial type data {:#X}, Data size {:#X}\n",
            data.position(),
            data.len()
        );*/

        if type_handle != 0 {
            // Now we need to read the data of the associated type using the "fillin" functions
            // For now I'm combining them into a single function
            let type_name = self.type_registry.get_mut(&type_handle).expect("a").to_owned();
            //println!("Filling in {} from {:#X}", type_name, data.position());
            self.fillin(data, &type_name)?;
        }
        if data.position() != data.len() {
            println!(
                "Finished at {:#X}, Data size {:#X}\n",
                data.position(),
                data.len()
            );
        }
        Ok(())
    }

    fn read_handle(&mut self, data: &mut Datagram) -> Result<u16, self::Error> {
        let type_handle = data.read_u16()?;

        // Found a new type, read its string and register it
        if !self.type_registry.contains_key(&type_handle) {
            //read_slice
            let length = data.read_u16()?;
            let slice = data.get_slice(length as usize)?;

            let type_name = core::str::from_utf8(&slice).map_err(|_| Error::InvalidType)?.to_owned();
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
            match self.objects_left {
                ObjectsLeft::ObjectCount { ref mut num_extra_objects } => {
                    *num_extra_objects -= 1;
                }
                _ => (),
            }
            return Ok(Some(object_id));
        }
        Ok(None)
    }

    //should really be using make_from_bam as an entrypoint
    fn fillin(&mut self, data: &mut Datagram, type_name: &str) -> Result<(), self::Error> {
        let node = match type_name {
            "AnimBundle" => PandaObject::AnimBundle(AnimBundle::create(self, data)?),
            "AnimBundleNode" => PandaObject::AnimBundleNode(AnimBundleNode::create(self, data)?),
            "AnimGroup" => PandaObject::AnimGroup(AnimGroup::create(self, data)?),
            "BillboardEffect" => PandaObject::BillboardEffect(BillboardEffect::create(self, data)?),
            "Character" => PandaObject::Character(Character::create(self, data)?),
            "CharacterJoint" => PandaObject::CharacterJoint(CharacterJoint::create(self, data)?),
            "CharacterJointBundle" => PandaObject::CharacterJointBundle(PartBundle::create(self, data)?),
            "CharacterJointEffect" => {
                PandaObject::CharacterJointEffect(CharacterJointEffect::create(self, data)?)
            }
            "CollisionCapsule" => PandaObject::CollisionCapsule(CollisionCapsule::create(self, data)?),
            "CollisionNode" => PandaObject::CollisionNode(CollisionNode::create(self, data)?),
            "CollisionTube" => PandaObject::CollisionCapsule(CollisionCapsule::create(self, data)?),
            "ColorAttrib" => PandaObject::ColorAttrib(ColorAttrib::create(self, data)?),
            "CullBinAttrib" => PandaObject::CullBinAttrib(CullBinAttrib::create(self, data)?),
            "CullFaceAttrib" => PandaObject::CullFaceAttrib(CullFaceAttrib::create(self, data)?),
            "DepthWriteAttrib" => PandaObject::DepthWriteAttrib(DepthWriteAttrib::create(self, data)?),
            "Geom" => PandaObject::Geom(Geom::create(self, data)?),
            "GeomNode" => PandaObject::GeomNode(GeomNode::create(self, data)?),
            "GeomTriangles" => PandaObject::GeomTriangles(GeomPrimitive::create(self, data)?),
            "GeomTristrips" => PandaObject::GeomTristrips(GeomPrimitive::create(self, data)?), /* TODO: cleanup GeomPrimitive */
            "GeomVertexArrayData" => {
                PandaObject::GeomVertexArrayData(GeomVertexArrayData::create(self, data)?)
            }
            "GeomVertexArrayFormat" => {
                PandaObject::GeomVertexArrayFormat(GeomVertexArrayFormat::create(self, data)?)
            }
            "GeomVertexData" => PandaObject::GeomVertexData(GeomVertexData::create(self, data)?),
            "GeomVertexFormat" => PandaObject::GeomVertexFormat(GeomVertexFormat::create(self, data)?),
            "InternalName" => PandaObject::InternalName(InternalName::create(self, data)?),
            "JointVertexTransform" => {
                PandaObject::JointVertexTransform(JointVertexTransform::create(self, data)?)
            }
            "ModelNode" => PandaObject::ModelNode(ModelNode::create(self, data)?),
            "ModelRoot" => PandaObject::ModelRoot(ModelNode::create(self, data)?),
            "PandaNode" => PandaObject::PandaNode(PandaNode::create(self, data)?),
            "PartGroup" => PandaObject::PartGroup(PartGroup::create(self, data)?),
            "RenderEffects" => PandaObject::RenderEffects(RenderEffects::create(self, data)?),
            "RenderState" => PandaObject::RenderState(RenderState::create(self, data)?),
            "Texture" => PandaObject::Texture(Texture::create(self, data)?),
            "TextureAttrib" => PandaObject::TextureAttrib(TextureAttrib::create(self, data)?),
            "TextureStage" => PandaObject::TextureStage(TextureStage::create(self, data)?),
            "TransformBlendTable" => {
                PandaObject::TransformBlendTable(TransformBlendTable::create(self, data)?)
            }
            "TransformState" => PandaObject::TransformState(TransformState::create(self, data)?),
            "TransparencyAttrib" => PandaObject::TransparencyAttrib(TransparencyAttrib::create(self, data)?),
            _ => todo!("{type_name}"),
        };
        //println!("{:#?}", node);
        self.nodes.push(node);
        Ok(())
    }
}
