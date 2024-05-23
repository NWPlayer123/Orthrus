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
use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::common::*;

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
pub(crate) type Result<T> = core::result::Result<T, Error>;

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

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct Header {
    pub(crate) version: Version,
    endian: Endian,
    /// BAM files after 6.27 support reading either floats or doubles (false/true)
    pub(crate) float_type: bool,
}

#[derive(Debug)]
enum ObjectCode {
    /// Includes an object definition, always paired with a Pop.
    Push,
    /// Paired with a Push in order to allow for nesting.
    Pop,
    /// Includes an object definition, does not change nesting level.
    Adjunct,
    /// List of object IDs that were deallocated by the sender, ???
    Remove,
    /// Additional file data that can be referenced by other objects, ???
    FileData,
}

impl TryFrom<u8> for ObjectCode {
    type Error = Error;

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => ObjectCode::Push,
            1 => ObjectCode::Pop,
            2 => ObjectCode::Adjunct,
            3 => ObjectCode::Remove,
            4 => ObjectCode::FileData,
            _ => return Err(Error::InvalidEnum),
        })
    }
}

/*
mod CollisionSolid {
    use bitflags::bitflags;
    bitflags! {
        #[derive(Debug)]
        pub(crate) struct Flags: u8 {
            /// Allows this to actually interact with other objects.
            const Tangible = 0x01;
            /// Lets us know we actually have an effective normal to worry about.
            const EffectiveNormal = 0x02;
            /// Lets us know if we need to update the GeomNode for visualization of the solid.
            const VisualizationStale = 0x04;
            /// Only checked if the collision solid is used as a collider. Uses the true normal
            /// instead.
            const IgnoreEffectiveNormal = 0x08;
            /// Lets us know we need to re-calculate the bounding volume.
            const InternalBoundsStale = 0x10;
        }
    }
}
*/

#[derive(Debug)]
pub struct BinaryAsset {
    /// Holds all BAM metadata needed for parsing
    pub(crate) header: Header,
    //TODO: combine these into a single value?
    /// Before BAM 6.21
    num_extra_objects: i32,
    /// Starting with BAM 6.21
    nesting_level: i32,
    /// Used if there are more than 0xFFFF ObjectIDs
    long_object_id: bool,
    registry: HashMap<u16, String>,
}

impl BinaryAsset {
    /// Latest revision of the BAM format. For more info, see [here](self#revisions).
    pub const CURRENT_VERSION: Version = Version { major: 6, minor: 45 };
    /// Unique identifier that tells us if we're reading a Panda3D Binary Object.
    pub const MAGIC: [u8; 6] = *b"pbj\0\n\r";
    /// Earliest supported revision of the BAM format. For more info, see [here](self#revisions).
    pub const MINIMUM_VERSION: Version = Version { major: 6, minor: 14 };

    #[inline]
    #[allow(dead_code)]
    fn read_header<T: EndianRead>(data: &mut T) -> Result<Header> {
        let version = Version { major: data.read_u16()?, minor: data.read_u16()? };

        ensure!(
            version.major == Self::CURRENT_VERSION.major
                && version.minor >= Self::MINIMUM_VERSION.minor
                && version.minor <= Self::CURRENT_VERSION.minor,
            InvalidVersionSnafu
        );

        let endian = match data.read_u8()? {
            0 => Endian::Big,
            1 => Endian::Little,
            _ => return Err(Error::InvalidEndian),
        };

        let float_type = match version.minor >= 27 {
            true => data.read_u8()? != 0,
            false => false,
        };

        Ok(Header { version, endian, float_type })
    }

    pub fn get_minor_version(&self) -> u16 {
        self.header.version.minor
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(input: P) -> Result<Self> {
        let data = std::fs::read(input)?;
        Self::load(data)
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<Self> {
        let mut data = DataCursor::new(input, Endian::Little);

        // Read the magic and make sure we're actually parsing a BAM file
        let mut magic = [0u8; 6];
        data.read_length(&mut magic)?;
        ensure!(magic == Self::MAGIC, InvalidMagicSnafu);

        // Load initial datagram and parse the header
        let mut datagram = Datagram::new(&mut data, Endian::Little, false)?;
        let header = Self::read_header(&mut *datagram)?;

        // Create the BinaryAsset instance so we can start constructing all the objects
        let mut bamfile = Self {
            header,
            num_extra_objects: 0,
            nesting_level: 0,
            long_object_id: false,
            registry: HashMap::new(),
        };

        // Read the initial object
        datagram = Datagram::new(&mut data, bamfile.header.endian, bamfile.header.float_type)?;
        bamfile.read_object(&mut datagram)?;

        //Parse objects until we run out, used before 6.21.
        while bamfile.num_extra_objects > 0 {
            println!("Datagram at {:#X}", data.position());
            datagram = Datagram::new(&mut data, bamfile.header.endian, bamfile.header.float_type)?;
            bamfile.read_object(&mut datagram)?;
            bamfile.num_extra_objects -= 1;
        }

        //Parse objects until we reach the initial nesting level, used starting with 6.21.
        while bamfile.nesting_level > 0 {
            println!("Datagram at {:#X}", data.position());
            datagram = Datagram::new(&mut data, bamfile.header.endian, bamfile.header.float_type)?;
            bamfile.read_object(&mut datagram)?;
        }

        println!("{:?}", bamfile.registry);

        Ok(bamfile)
    }

    fn read_object(&mut self, data: &mut Datagram) -> Result<()> {
        // Try to read and handle ObjectCode control flow
        if self.header.version.minor >= 21 {
            // If we're past 6.21, "BamObjectCode" control flow codes are in the data stream
            let object_code = data.read_u8()?.try_into()?;
            println!("Object Code: {:?}", object_code);
            match object_code {
                ObjectCode::Push => {
                    self.nesting_level += 1;
                }
                ObjectCode::Pop => {
                    self.nesting_level -= 1;
                    return Ok(());
                }
                ObjectCode::Adjunct => {}
                _ => {
                    todo!(
                        "Remove and FileData are unimplemented, need a test case, pls message me."
                    )
                }
            }
        }

        // Check the type handle, see if we need to register any new types
        let type_handle = self.read_handle(data)?;
        // Read the Object ID and process it
        let object_id = self.read_object_id(data)?;
        println!("Object ID {}", object_id);
        println!(
            "Finished at {:#X}, Data size {:#X}\n",
            data.position(),
            data.len()
        );

        if type_handle != 0 {
            // Now we need to read the data of the associated type using the "fillin" functions
            // For now I'm combining them into a single function
            let type_name = self.registry.get_mut(&type_handle).expect("a").to_owned();
            println!("Filling in {} from {:#X}", type_name, data.position());
            self.fillin(data, &type_name)?;
        }
        println!(
            "Finished at {:#X}, Data size {:#X}\n",
            data.position(),
            data.len()
        );
        Ok(())
    }

    fn read_handle(&mut self, data: &mut Datagram) -> Result<u16> {
        let type_handle = data.read_u16()?;

        // Found a new type, read its string and register it
        if !self.registry.contains_key(&type_handle) {
            //read_slice
            let length = data.read_u16()?;
            let slice = data.get_slice(length as usize)?;

            let type_name =
                core::str::from_utf8(&slice).map_err(|_| Error::InvalidType)?.to_owned();
            println!("Registering Type {type_name} -> {type_handle}");
            self.registry.insert(type_handle, type_name);

            //Check for any parent classes we need to register
            let parent_count = data.read_u8()?;
            println!("Parent Count: {parent_count}");
            for _ in 0..parent_count {
                self.read_handle(data)?;
            }
        }

        Ok(type_handle)
    }

    fn read_object_id(&mut self, data: &mut Datagram) -> Result<u32> {
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

    pub(crate) fn read_pointer(&mut self, data: &mut Datagram) -> Result<Option<u32>> {
        let object_id = self.read_object_id(data)?;
        println!("Object ID ptrto {}", object_id);
        if object_id != 0 {
            if self.header.version.minor < 21 {
                self.num_extra_objects += 1;
            }
            return Ok(Some(object_id));
        }
        Ok(None)
    }

    //should really be using make_from_bam as an entrypoint
    fn fillin(&mut self, data: &mut Datagram, type_name: &str) -> Result<()> {
        match type_name {
            //3
            "TypedObject" => {
                // Base class, nothing to fill in
            }
            //4
            "ReferenceCount" => {
                // Base class, nothing to fill in
            }
            //12
            "Namable" => {
                // Base class, nothing to fill in
            }
            //48
            "TypedWritable" => {
                // Base class, everything derived from here.
                //
                // See typedWritable.h.template, all BAM files use register_with_read_factory,
                // make_from_bam, fillin, complete_pointers, finalize, _num_GenericPointers, and
                // write_datagram
                //
                // In this design, I'm combining:
                // * register_with_read_factory, make_from_bam, and fillin into Object::create
                // * complete_pointers, finalize, and _num_GenericPointers into Object::finalize
                // * write_datagram into Object::write
            }
            //49
            "PandaNode" => {
                //TypedWritable::fillin()
                //remove_all_children()
                let node = crate::nodes::panda_node::PandaNode::create(self, data)?;
                println!("{:#?}", node);
            }
            //52
            "TypedWritableReferenceCount" => {
                // Base class, nothing to fill in
            }
            //53
            "CachedTypedWritableReferenceCount" => {
                // Base class, nothing to fill in
            }
            //54
            "CopyOnWriteObject" => {
                // Base class, nothing to fill in
            }
            //93
            "RenderAttrib" => {
                // Base class, other Attrib derive from this
            }
            //101
            "ColorAttrib" => {
                let node = crate::nodes::color_attrib::ColorAttrib::create(self, data)?;
                println!("{:#?}", node);
            }
            //108
            "CullBinAttrib" => {
                let node = crate::nodes::cull_bin_attrib::CullBinAttrib::create(self, data)?;
                println!("{:#?}", node);
            }
            //124
            "GeomNode" => {
                let node = crate::nodes::geom_node::GeomNode::create(self, data)?;
                println!("{:#?}", node);
            }
            //136
            "ModelNode" => {
                let node = crate::nodes::model_node::ModelNode::create(self, data)?;
                println!("{:#?}", node);
            }
            //137
            "ModelRoot" => {
                Self::fillin(self, data, "ModelNode")?;
            }
            //149
            "RenderEffects" => {
                let node = crate::nodes::render_effects::RenderEffects::create(self, data)?;
                println!("{:#?}", node);
            }
            //151
            "NodeCachedReferenceCount" => {
                // Base class, nothing to fill in
            }
            //152
            "RenderState" => {
                let node = crate::nodes::render_state::RenderState::create(self, data)?;
                println!("{:#?}", node);
            }
            //166
            "TextureAttrib" => {
                let node = crate::nodes::texture_attrib::TextureAttrib::create(self, data)?;
                println!("{:#?}", node);
            }
            //168
            "TransformState" => {
                let node = crate::nodes::transform_state::TransformState::create(self, data)?;
                println!("{:#?}", node);
            }
            //169
            "TransparencyAttrib" => {
                let node =
                    crate::nodes::transparency_attrib::TransparencyAttrib::create(self, data)?;
                println!("{:#?}", node);
            }
            //201
            "Texture" => {
                let node = crate::nodes::texture::Texture::create(self, data)?;
                println!("{:#?}", node);
            }
            //237
            "CollisionSolid" => {}
            //249
            "CollisionSphere" => {}
            //254
            "CollisionPlane" => {}
            //255
            "CollisionPolygon" => {}
            //257
            "CollisionNode" => {}
            //328
            "Geom" => {}
            //335
            "GeomPrimitive" => {}
            //341
            "GeomTriangles" => {}
            //343
            "GeomTristrips" => {}
            //344
            "GeomVertexArrayData" => {}
            //347
            "GeomVertexArrayFormat" => {}
            //348
            "GeomVertexData" => {}
            //354
            "GeomVertexFormat" => {
                //TypedWritableReferenceCount::fillin
            }
            //356
            "InternalName" => {}
            //372
            "TextureStage" => {
                let texture_stage = crate::nodes::texture_stage::TextureStage::create(self, data)?;
                println!("{:#?}", texture_stage);
            }
            _ => todo!("{type_name}"),
        }
        /*match type_name {
            //3
            "TypedObject" => {
                //empty, used by TypedWritable
            }
            //4
            "ReferenceCount" => {
                //empty, used by TypedWritable
            }
            //12
            "Namable" => {
                //empty, used by TypedWritable
            }
            //48
            "TypedWritable" => {
                //empty, base class
            }
            //49
            "PandaNode" => {
                self.fillin(data, "TypedWritable")?;

                //remove_all_children
                let length = data.read_u16()?;
                let slice = data.get_slice(length as usize)?;
                let name = core::str::from_utf8(&slice).map_err(|_| Error::InvalidType)?;
                println!("PandaNode name: {:?}", name);

                //PandaNode::CData::fillin
                //state
                self.read_pointer(data)?;
                //transform
                self.read_pointer(data)?;
                //effects
                self.read_pointer(data)?;

                if self.header.version.minor < 2 {
                    let draw_mask = data.read_u32()?;
                    //some additional processing that I don't need to set
                    println!("PandaNode Draw Mask: {:#08X}", draw_mask);
                } else {
                    let draw_control_mask = data.read_u32()?;
                    let draw_show_mask = data.read_u32()?;
                    println!(
                        "PandaNode Draw Mask: Control {:#08X}, Show {:#08X}",
                        draw_control_mask, draw_show_mask
                    );
                }

                let into_collide_mask = data.read_u32()?;
                println!("PandaNode Collide Mask: {:#08X}", into_collide_mask);

                if self.header.version.minor >= 19 {
                    let bounds_type: BoundingVolume::BoundsType = data.read_u8()?.try_into()?;
                    println!("PandaNode BoundsType: {:?}", bounds_type);
                }

                let num_tags = data.read_u32()?;
                for _ in 0..num_tags {
                    let mut length = data.read_u16()?;
                    let mut slice = data.get_slice(length as usize)?;
                    let key =
                        core::str::from_utf8(&slice).map_err(|_| Error::InvalidType)?.to_owned();
                    length = data.read_u16()?;
                    slice = data.get_slice(length as usize)?;
                    let value =
                        core::str::from_utf8(&slice).map_err(|_| Error::InvalidType)?.to_owned();
                    println!("PandaNode Key-Value {} -> {}", key, value);
                }

                //fillin_up_list up
                let num_parents = data.read_u16()?;
                println!("PandaNode Parent Count: {}", num_parents);
                for _ in 0..num_parents {
                    self.read_pointer(data)?;
                }
                //fillin_down_list down
                let mut num_children = data.read_u16()?;
                println!("PandaNode Children Count: {}", num_children);
                for _ in 0..num_children {
                    self.read_pointer(data)?;
                    let sort = data.read_u32()?;
                    println!("Sort {}", sort);
                }
                //fillin_down_list stashed
                num_children = data.read_u16()?;
                println!("PandaNode Stashed Count: {}", num_children);
                for _ in 0..num_children {
                    self.read_pointer(data)?;
                    let sort = data.read_u32()?;
                    println!("Sort {}", sort);
                }
            }
            //52
            "TypedWritableReferenceCount" => {
                //empty, used by CachedTypedWritableReferenceCount
            }
            //53
            "CachedTypedWritableReferenceCount" => {
                //empty, used by NodeCachedReferenceCount
            }
            //54
            "CopyOnWriteObject" => {
                //empty, used by CollisionSolid
            }
            //124
            "GeomNode" => {
                self.fillin(data, "PandaNode")?;

                //fill in cycle data
                let num_geoms = data.read_u16()?;
                println!("GeomNode Count: {num_geoms}");
                for _ in 0..num_geoms {
                    //Geom, RenderState?
                    self.read_pointer(data)?;
                    self.read_pointer(data)?;
                }
            }
            //136
            "ModelNode" => {
                self.fillin(data, "PandaNode")?;

                let preserve_transform: PreserveTransform = data.read_u8()?.try_into()?;
                //SceneGraphReducer::AttribTypes that can't be overwritten
                let preserve_attributes = data.read_u16()?;
                println!(
                    "ModelNode: Transform {:?}, Attributes {:#X}",
                    preserve_transform, preserve_attributes
                );
            }
            //137
            "ModelRoot" => {
                self.fillin(data, "ModelNode")?;
            }
            //149
            "RenderEffects" => {
                self.fillin(data, "TypedWritable")?;

                let num_effects = data.read_u16()?;
                for _ in 0..num_effects {
                    //self.read_pointer(data)?;
                }
                println!("RenderEffects count: {num_effects}");

                //reserve this many effects
            }
            //151
            "NodeCachedReferenceCount" => {
                //empty, used by RenderState
            }
            //152
            "RenderState" => {
                self.fillin(data, "TypedWritable")?;

                let num_attributes = data.read_u16()?;
                let mut read_overrides = vec![0i32; num_attributes as usize];

                for _ in 0..num_attributes {
                    self.read_pointer(data)?;
                    let read_override = data.read_i32()?;
                    read_overrides.push(read_override);
                }
                println!("RenderState read_overrides: {:?}", read_overrides);
            }
            //168
            "TransformState" => {
                self.fillin(data, "TypedWritable")?;

                let flags = TransformState::Flags::from_bits_truncate(data.read_u32()?);
                println!("TransformState flags: {:?}", flags);
                if flags.contains(TransformState::Flags::ComponentsGiven) {
                    let position = [data.read_i32()?, data.read_i32()?, data.read_i32()?];
                    println!("Position: {:?}", position);
                    if flags.contains(TransformState::Flags::QuaternionGiven) {
                        let quaternion = [
                            data.read_i32()?,
                            data.read_i32()?,
                            data.read_i32()?,
                            data.read_i32()?,
                        ];
                        println!("Quaternion: {:?}", quaternion);
                    } else {
                        let hpr = [data.read_i32()?, data.read_i32()?, data.read_i32()?];
                        println!("Heading, Pitch, Roll: {:?}", hpr);
                    }
                    let scale = [data.read_i32()?, data.read_i32()?, data.read_i32()?];
                    println!("Scale: {:?}", scale);
                    let shear = [data.read_i32()?, data.read_i32()?, data.read_i32()?];
                    println!("Shear: {:?}", shear);
                }

                if flags.contains(TransformState::Flags::MatrixKnown) {
                    //4x4 matrix
                    if self.header.double {
                        let matrix = [
                            [
                                data.read_f64()?,
                                data.read_f64()?,
                                data.read_f64()?,
                                data.read_f64()?,
                            ],
                            [
                                data.read_f64()?,
                                data.read_f64()?,
                                data.read_f64()?,
                                data.read_f64()?,
                            ],
                            [
                                data.read_f64()?,
                                data.read_f64()?,
                                data.read_f64()?,
                                data.read_f64()?,
                            ],
                            [
                                data.read_f64()?,
                                data.read_f64()?,
                                data.read_f64()?,
                                data.read_f64()?,
                            ],
                        ];
                        println!("Matrix: {:?}", matrix);
                    } else {
                        let matrix = [
                            [
                                data.read_f32()?,
                                data.read_f32()?,
                                data.read_f32()?,
                                data.read_f32()?,
                            ],
                            [
                                data.read_f32()?,
                                data.read_f32()?,
                                data.read_f32()?,
                                data.read_f32()?,
                            ],
                            [
                                data.read_f32()?,
                                data.read_f32()?,
                                data.read_f32()?,
                                data.read_f32()?,
                            ],
                            [
                                data.read_f32()?,
                                data.read_f32()?,
                                data.read_f32()?,
                                data.read_f32()?,
                            ],
                        ];
                        println!("Matrix: {:?}", matrix);
                    }
                }
            }
            //237
            ""
            //257
            "CollisionNode" => {
                self.fillin(data, "PandaNode")?;

                let mut num_solids: u32 = data.read_u16()? as u32;
                if num_solids == 0xFFFF {
                    num_solids = data.read_u32()?;
                }

                for _ in 0..num_solids {
                    self.read_pointer(data)?;
                }
            }
            _ => todo!("{}", type_name),
        }*/
        Ok(())
    }
}
