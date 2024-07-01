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

        // Now we can try to actually parse the data, first node = root node (should always be a ModelRoot!)
        println!("{} {:?}", 1, &bamfile.nodes[1]);
        bamfile.recurse_nodes(1);

        //println!("{:?}", bamfile.type_registry);

        Ok(bamfile)
    }

    //TODO: rewriting EggSaver::add_subgraph
    fn _walk_tree(&self, node_index: usize) {
        println!("{} {:?}", node_index, &self.nodes[node_index]);
        match &self.nodes[node_index] {
            PandaObject::ModelRoot(root) => {
                // We're currently at the root node, so just iterate all children
                assert!(root.node.child_refs.len() == 1);
                self._walk_tree(root.node.child_refs[0].0 as usize);
            }
            PandaObject::Character(node) => {
                // We probably got called by the ModelRoot, just keep iterating children
                //Sanity check, Characters have geom data and then joint data?
                assert!(node.node.node.child_refs.len() == 2);
                for child_index in &node.node.node.child_refs {
                    self._walk_tree(child_index.0 as usize);
                }
                //TODO: we should treat this as its own entity in the Scene
            }
            PandaObject::PandaNode(node) => {
                // Just a generic grouping node, keep iterating children
                for child_index in &node.child_refs {
                    self._walk_tree(child_index.0 as usize);
                }
            }
            PandaObject::GeomNode(node) => {
                // We've hit a GeomNode, so handle its geometry
                println!("// {}", node.node.name);
                for geom_index in &node.geom_refs {
                    // First, grab the actual geometry data
                    self._walk_tree(geom_index.0 as usize);
                    // Then, process RenderEffects so we can have a proper texture
                    self._walk_tree(geom_index.1 as usize);
                    println!("commands.spawn((PbrBundle {{mesh: meshes.add(mesh), material: materials.add(material), ..default()}}, CustomUV));");
                    println!("");
                }
            }
            PandaObject::Geom(node) => {
                assert!(node.primitive_refs.len() == 1);
                // First, figure out what type of primitive we're interpreting
                self._walk_tree(node.primitive_refs[0] as usize);
                // Then, let's get the actual vertex data
                self._walk_tree(node.data_ref as usize);
            }
            PandaObject::GeomTristrips(node) => {
                // We got called from a Geom primitive, so print the first part of the built mesh
                println!("let mut mesh = Mesh::new(PrimitiveTopology::TriangleStrip, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);");
                let index_node = &self.nodes[node.vertices_ref.unwrap() as usize];
                match index_node {
                    PandaObject::GeomVertexArrayData(data) => {
                        //TODO: more error handling? For now, we can just use the Tristrip
                        match node.index_type {
                            NumericType::U16 => {
                                let buffer = &data.buffer;
                                let mut indices = Vec::with_capacity(buffer.len() / 2 as usize);
                                let mut cursor = DataCursorRef::new(buffer, Endian::Little);
                                for _ in 0..(buffer.len() / 2) {
                                    indices.push(cursor.read_u16().unwrap());
                                }
                                println!("mesh.insert_indices(Indices::U16(vec!{:?}));", indices);
                            }
                            _ => todo!("Unsupported GeomTristrips index type!"),
                        }
                    }
                    _ => panic!("Unexpected GeomTristrip data!"),
                }
            }
            PandaObject::GeomTriangles(node) => {
                // We got called from a Geom primitive, so print the first part of the built mesh
                println!("let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);");
                let index_node = &self.nodes[node.vertices_ref.unwrap() as usize];
                match index_node {
                    PandaObject::GeomVertexArrayData(data) => {
                        //TODO: more error handling? For now, we can just use the Tristrip
                        match node.index_type {
                            NumericType::U16 => {
                                let buffer = &data.buffer;
                                let mut indices = Vec::with_capacity(buffer.len() / 2 as usize);
                                let mut cursor = DataCursorRef::new(buffer, Endian::Little);
                                for _ in 0..(buffer.len() / 2) {
                                    indices.push(cursor.read_u16().unwrap());
                                }
                                println!("mesh.insert_indices(Indices::U16(vec!{:?}));", indices);
                            }
                            _ => todo!("Unsupported GeomTristrips index type!"),
                        }
                    }
                    _ => panic!("Unexpected GeomTristrip data!"),
                }
            }
            PandaObject::GeomVertexData(node) => {
                // We got called from a Geom primitive, we can use this node to get the rest of the data we
                // need
                assert!(node.array_refs.len() == 1 || node.array_refs.len() == 2);
                for array_ref in &node.array_refs[..=0] {
                    let array_data = &self.nodes[*array_ref as usize];
                    match array_data {
                        PandaObject::GeomVertexArrayData(data) => {
                            // Grab the buffer data so we can interpret it
                            let mut buffer = DataCursorRef::new(&data.buffer, Endian::Little);
                            let array_format = &self.nodes[data.array_format_ref as usize];
                            let format = match array_format {
                                PandaObject::GeomVertexArrayFormat(format) => format,
                                _ => panic!("Unexpected GeomVertexArrayFormat!"),
                            };

                            // Handle each column individually, run its stride, and print the relevant data
                            for column in &format.columns {
                                match column.contents {
                                    Contents::Point => {
                                        println!("mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float32x3(vec![");
                                    }
                                    Contents::TexCoord => {
                                        println!("mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(vec![");
                                    }
                                    Contents::Color => {
                                        println!("mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, VertexAttributeValues::Float32x4(vec![");
                                    }
                                    _ => {
                                        //TODO: GeomVertexData::get_transform_blend_table and
                                        // Character::copy_geom
                                        println!("{:?}", column.contents);
                                        todo!("Haven't implemented other vertex contents yet");
                                    }
                                }
                                let start = 0;
                                for stride in start..start + (buffer.len() / format.stride as usize) {
                                    buffer.set_position(
                                        stride * format.stride as usize + column.start as usize,
                                    );
                                    let mut entry = match column.numeric_type {
                                        NumericType::F32 => {
                                            let mut vec = Vec::with_capacity(column.num_values as usize);
                                            for _ in 0..column.num_values {
                                                vec.push(buffer.read_f32().unwrap());
                                            }
                                            vec
                                        }
                                        NumericType::PackedDABC => {
                                            let data = buffer.read_u32().unwrap();
                                            let a = ((data >> 24) & 0xFF) as f32 / 255.0;
                                            let r = ((data >> 16) & 0xFF) as f32 / 255.0;
                                            let g = ((data >> 8) & 0xFF) as f32 / 255.0;
                                            let b = ((data >> 0) & 0xFF) as f32 / 255.0;
                                            vec![r, g, b, a]
                                        }
                                        _ => todo!("Non-F32/PackedDABC data not implemented yet!"),
                                    };
                                    match column.contents {
                                        Contents::Point => {
                                            println!("{:?},", entry);
                                        }
                                        Contents::TexCoord => {
                                            // Have to flip Y for OpenGL nonsense
                                            entry[1] = 1.0 - entry[1];
                                            println!("{:?},", entry);
                                        }
                                        Contents::Color => {
                                            // Needs to be Linear RGB(A)!!!
                                            println!(
                                                "Color::rgba({:?}, {:?}, {:?}, {:?}).as_linear_rgba_f32(),",
                                                entry[0], entry[1], entry[2], entry[3]
                                            );
                                        }
                                        _ => todo!("Haven't implemented other vertex contents yet"),
                                    }
                                }
                                println!("]));");
                            }
                        }
                        _ => panic!("Unexpected GeomVertexArrayData!"),
                    }
                }
            }
            PandaObject::RenderState(node) => {
                println!("let material = StandardMaterial {{");
                for attrib in &node.attrib_refs {
                    let attrib = &self.nodes[attrib.0 as usize];
                    //println!("{:?}", attrib);
                    match attrib {
                        PandaObject::ColorAttrib(attrib) => {
                            //TODO: handle color_type?
                            println!(
                                "base_color: Color::rgba_from_array([{:?}, {:?}, {:?}, {:?}]),",
                                attrib.color.x, attrib.color.y, attrib.color.z, attrib.color.w
                            );
                        }
                        PandaObject::TextureAttrib(attrib) => {
                            assert!(attrib.on_stages.len() == 1);
                            match &self.nodes[attrib.on_stages[0].texture_ref as usize] {
                                PandaObject::Texture(texture) => {
                                    println!(
                                        "base_color_texture: Some(asset_server.load(\"{}\")),",
                                        texture.filename
                                    );
                                }
                                _ => panic!("Unexpected Texture node!"),
                            }
                        }
                        PandaObject::TransparencyAttrib(attrib) => match attrib.mode {
                            TransparencyMode::Dual => {
                                println!("alpha_mode: AlphaMode::Blend,");
                            }
                            TransparencyMode::Alpha => {
                                println!("alpha_mode: AlphaMode::Blend,");
                            }
                            _ => panic!("Haven't implemented other transparency modes yet!"),
                        },
                        PandaObject::CullFaceAttrib(attrib) => match attrib.mode {
                            CullMode::None => {
                                println!("cull_mode: None,");
                            }
                            CullMode::Clockwise => {
                                println!("cull_mode: Some(Face::Front),");
                            }
                            CullMode::CounterClockwise => {
                                println!("cull_mode: Some(Face::Back),");
                            }
                            _ => todo!("Haven't implemented that cull face mode!"),
                        },
                        PandaObject::CullBinAttrib(_) => {} /* TODO: figure out how to implement this in a */
                        // bevy material?
                        PandaObject::DepthWriteAttrib(_) => {} /* TODO: custom material for turning off */
                        // depth writes! need custom pipeline
                        _ => todo!("Haven't added support for this attrib yet!"),
                    }
                }
                println!("unlit: true,");
                println!("..default()");
                println!("}};");
            }
            _ => (),
        }
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
