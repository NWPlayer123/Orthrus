use std::path::PathBuf;

use crate::nodes::prelude::*;
use crate::prelude::*;

use bevy_asset::io::Reader;
use bevy_asset::{Asset, AssetLoader, AsyncReadExt, Handle, LoadContext};
use bevy_log::prelude::*;
use bevy_pbr::StandardMaterial;
use bevy_reflect::TypePath;
use bevy_render::alpha::AlphaMode;
use bevy_render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy_render::render_asset::RenderAssetUsages;
use bevy_scene::Scene;
use orthrus_core::prelude::*;
use serde::{Deserialize, Serialize};

//TODO: create struct for passing around all the data in these signatures? Also refactor to actually return errors

#[derive(Asset, Debug, Default, TypePath)]
pub struct BinaryAsset {
    pub scenes: Vec<Handle<Scene>>,
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
}

impl BinaryAsset {
    /// This function is used to recursively convert all child nodes
    pub(crate) fn recurse_nodes(
        &mut self, settings: &BamLoaderSettings, context: &mut LoadContext, asset: &crate::bam::BinaryAsset,
        node_index: usize,
    ) {
        //TODO: has_decal, joint_map
        match &asset.nodes[node_index] {
            PandaObject::ModelRoot(node) => {
                println!("{} {:?}", node_index, &asset.nodes[node_index]);
                for child in &node.node.child_refs {
                    self.convert_node(settings, context, asset, child.0 as usize);
                }
            }
            PandaObject::ModelNode(node) => {
                println!("{} {:?}", node_index, &asset.nodes[node_index]);
                for child in &node.node.child_refs {
                    self.convert_node(settings, context, asset, child.0 as usize);
                }
            }
            PandaObject::PandaNode(node) => {
                println!("{} {:?}", node_index, &asset.nodes[node_index]);
                for child in &node.child_refs {
                    self.convert_node(settings, context, asset, child.0 as usize);
                }
            }
            PandaObject::GeomNode(node) => {
                for child in &node.node.child_refs {
                    self.convert_node(settings, context, asset, child.0 as usize);
                }
            }
            PandaObject::Character(node) => {
                for child in &node.node.node.child_refs {
                    self.convert_node(settings, context, asset, child.0 as usize);
                }
            }
            _ => (),
        }
    }

    /// This function is used to process node data and convert it to a useful format
    // TODO: merge with recurse_nodes?
    fn convert_node(
        &mut self, settings: &BamLoaderSettings, context: &mut LoadContext, asset: &crate::bam::BinaryAsset,
        node_index: usize,
    ) {
        match &asset.nodes[node_index] {
            PandaObject::GeomNode(_) => {
                self.convert_geom_node(settings, context, asset, node_index);
            }
            PandaObject::Character(_) => {
                self.convert_character_node(settings, context, asset, node_index);
            }
            _ => {
                // Otherwise, it's just a generic node and we need to keep recursing
                // TODO: register parent/child hierarchy?
                self.recurse_nodes(settings, context, asset, node_index)
            }
        }
    }

    fn convert_geom_node(
        &mut self, settings: &BamLoaderSettings, context: &mut LoadContext, asset: &crate::bam::BinaryAsset,
        node_index: usize,
    ) {
        let node = match &asset.nodes[node_index] {
            PandaObject::GeomNode(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        // TODO: handle node properties like billboard, transformstate, renderstate whenever they come up
        // TODO: whenever something actually has a decal, all this needs refactoring

        // (Geom, RenderState)
        for geom_ref in &node.geom_refs {
            let geom = match &asset.nodes[geom_ref.0 as usize] {
                PandaObject::Geom(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            let render_state = match &asset.nodes[geom_ref.1 as usize] {
                PandaObject::RenderState(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            println!("{} {:?}", geom_ref.0, geom);
            println!("{} {:?}", geom_ref.1, render_state);
            for primitive_ref in &geom.primitive_refs {
                //TODO: get node and decompose it? We also should pass vertex_data
                self.convert_primitive(
                    settings,
                    context,
                    asset,
                    render_state,
                    geom.data_ref as usize,
                    *primitive_ref as usize,
                );
            }
        }

        // After we've done our processing, check any child nodes
        self.recurse_nodes(settings, context, asset, node_index);
    }

    fn convert_primitive(
        &mut self, settings: &BamLoaderSettings, context: &mut LoadContext, asset: &crate::bam::BinaryAsset,
        render_state: &RenderState, data_index: usize, node_index: usize,
    ) {
        // First, load the GeomPrimitive and all the associated GeomVertex indices data
        let primitive_node = &asset.nodes[node_index];
        let primitive = match primitive_node {
            PandaObject::GeomTristrips(node) => node,
            PandaObject::GeomTriangles(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let vertex_data = match &asset.nodes[data_index] {
            PandaObject::GeomVertexData(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let vertex_format = match &asset.nodes[vertex_data.format_ref as usize] {
            PandaObject::GeomVertexFormat(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let array_data = match &asset.nodes[primitive.vertices_ref.unwrap() as usize] {
            PandaObject::GeomVertexArrayData(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let array_format = match &asset.nodes[array_data.array_format_ref as usize] {
            PandaObject::GeomVertexArrayFormat(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        println!("New Primitive {} {:?}", node_index, primitive);
        println!("{} {:?}", data_index, vertex_data);
        println!("{} {:?}", vertex_data.format_ref, vertex_format);
        println!("{} {:?}", primitive.vertices_ref.unwrap(), array_data);
        println!("{} {:?}", array_data.array_format_ref, array_format);

        // If we have a RenderState with attributes, we need to create a material
        if !render_state.attrib_refs.is_empty() {
            let label = format!("Material{}", self.materials.len());
            self.materials.push(context.labeled_asset_scope(label, |context| {
                let mut material = StandardMaterial::default();
                material.unlit = true; //TODO: disable this? Toontown specific

                for attrib_ref in &render_state.attrib_refs {
                    println!(
                        "Render State {} {:?}",
                        attrib_ref.0, &asset.nodes[attrib_ref.0 as usize]
                    );
                    match &asset.nodes[attrib_ref.0 as usize] {
                        PandaObject::TransparencyAttrib(attrib) => {
                            material.alpha_mode = match attrib.mode {
                                TransparencyMode::None => AlphaMode::Opaque,
                                TransparencyMode::Alpha => AlphaMode::Blend,
                                TransparencyMode::PremultipliedAlpha => AlphaMode::Premultiplied,
                                TransparencyMode::Binary => AlphaMode::Mask(0.5),
                                TransparencyMode::Dual => AlphaMode::Blend, //TODO: actually verify this?
                                _ => {
                                    warn!(
                                        "Encountered a TransparencyMode using multisamples, unimplemented!"
                                    );
                                    AlphaMode::Opaque
                                }
                            };
                        }
                        PandaObject::TextureAttrib(attrib) => {
                            if attrib.on_stages.len() > 1 {
                                warn!("Multiple TextureAttrib Stages! Something may be broken!");
                            }
                            for stage in &attrib.on_stages {
                                let texture_stage = match &asset.nodes[stage.texture_stage_ref as usize] {
                                    PandaObject::TextureStage(node) => node,
                                    _ => panic!("Something has gone horribly wrong!"),
                                };
                                let texture = match &asset.nodes[stage.texture_ref as usize] {
                                    PandaObject::Texture(node) => node,
                                    _ => panic!("Something has gone horribly wrong!"),
                                };
                                // TODO: actually handle TextureStages since I don't have a good way to right now
                                if texture_stage.name != "default" {
                                    warn!("Unimplemented TextureStage behavior!");
                                }

                                // Process the image path, make sure it's png
                                let mut image_path = PathBuf::from(texture.filename.clone());
                                println!("fuck {image_path:?}");
                                println!("{:?}", settings);
                                if !settings.material_path.is_empty() {
                                    let mut new_path = PathBuf::from(settings.material_path.clone());
                                    println!("fuckshit {new_path:?}");
                                    new_path.push(image_path.file_name().unwrap());
                                    println!("{new_path:?}");
                                    image_path = new_path;
                                }
                                image_path.set_extension("png");
                                println!("{image_path:?}");
                                material.base_color_texture = Some(context.load(image_path));
                                println!("TextureAttrib {} {:?}", stage.texture_stage_ref, texture_stage);
                                println!("TextureAttrib {} {:?}", stage.texture_ref, texture);
                            }
                        }
                        _ => warn!("Unimplemented Attribute!"), //_ => panic!("Unknown RenderState attribute!"),
                    }
                }

                

                println!("Material{}: {:?}", self.materials.len(), material);
                material
            }));
        }

        let _num_primitives = match primitive_node {
            PandaObject::GeomTristrips(node) => {
                // This is a complex primitive, so we need to load ends_ref and get the length of the array
                asset.arrays[node.ends_ref.unwrap() as usize].len()
            }
            PandaObject::GeomTriangles(node) => {
                // This is a simple primitive, need to check if num_vertices is defined, otherwise use stride
                match node.num_vertices {
                    -1 => array_data.buffer.len() / array_format.stride as usize,
                    num_vertices => num_vertices as usize,
                }
            }
            _ => panic!("Something has gone horribly wrong!"),
        };

        let label = format!("Mesh{}", self.meshes.len());
        self.meshes.push(context.labeled_asset_scope(label, |_context| {
            let topology = match primitive_node {
                PandaObject::GeomTristrips(_) => PrimitiveTopology::TriangleStrip,
                PandaObject::GeomTriangles(_) => PrimitiveTopology::TriangleList,
                _ => panic!("Unimplemented attribute!"),
            };
            //TODO: custom Mesh usages?
            let mut mesh = Mesh::new(topology, RenderAssetUsages::default());

            //First, let's process the GeomVertex data we got above, which should always just be a list of indices
            assert!(array_format.num_columns == 1);
            let internal_name = match &asset.nodes[array_format.columns[0].name_ref as usize] {
                PandaObject::InternalName(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            assert!(internal_name.name == "index");
            assert!(array_format.columns[0].numeric_type == NumericType::U16);
            let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);
            let mut mesh_indices = Vec::with_capacity(data.len() / 2);
            for _ in 0..mesh_indices.capacity() {
                mesh_indices.push(data.read_u16().unwrap());
            }
            mesh.insert_indices(Indices::U16(mesh_indices));

            // Now process all sub-array data. If there's more than one array, we're using a blend table of some sort
            for n in 0..vertex_data.array_refs.len() {
                let array_data = match &asset.nodes[vertex_data.array_refs[n] as usize] {
                    PandaObject::GeomVertexArrayData(node) => node,
                    _ => panic!("Something has gone horribly wrong!"),
                };
                let array_format = match &asset.nodes[vertex_format.array_refs[n] as usize] {
                    PandaObject::GeomVertexArrayFormat(node) => node,
                    _ => panic!("Something has gone horribly wrong!"),
                };
                println!("Sub-Array {}: {} {:?}", n, vertex_data.array_refs[n], array_data);
                println!("{} {:?}", vertex_format.array_refs[n], array_format);
                let num_primitives = array_data.buffer.len() / array_format.stride as usize;
                println!("Number of primitives: {}", num_primitives);
                let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);
                for column in &array_format.columns {
                    // For each sub-array, process all columns individually
                    let internal_name = match &asset.nodes[column.name_ref as usize] {
                        PandaObject::InternalName(node) => node,
                        _ => panic!("Something has gone horribly wrong!"),
                    };

                    println!("Name: {}", internal_name.name);

                    data.set_position(column.start as usize);
                    match internal_name.name.as_str() {
                        "vertex" => {
                            assert!(column.num_components == 3);
                            assert!(column.contents == Contents::Point);
                            assert!(column.numeric_type == NumericType::F32);
                            let mut vertex_data = Vec::with_capacity(num_primitives);
                            for n in 0..num_primitives {
                                data.set_position(column.start as usize + (array_format.stride as usize * n));
                                vertex_data.push([
                                    data.read_f32().unwrap(),
                                    data.read_f32().unwrap(),
                                    data.read_f32().unwrap(),
                                ]);
                            }
                            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_data);
                        }
                        "texcoord" => {
                            assert!(column.num_components == 2);
                            assert!(column.contents == Contents::TexCoord);
                            assert!(column.numeric_type == NumericType::F32);
                            let mut texcoord_data = Vec::with_capacity(num_primitives);
                            for n in 0..num_primitives {
                                data.set_position(column.start as usize + (array_format.stride as usize * n));
                                texcoord_data.push([
                                    data.read_f32().unwrap(),
                                    1.0 - data.read_f32().unwrap(),
                                ]);
                            }
                            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, texcoord_data);
                        }
                        "transform_blend" => {
                            //TODO: refactoring
                        }
                        _ => panic!("Unimplemented Vertex Type!"),
                    }
                }
            }

            println!("Mesh{}: {:?}", self.meshes.len(), mesh);
            mesh
        }));
    }

    fn convert_character_node(
        &mut self, settings: &BamLoaderSettings, context: &mut LoadContext, asset: &crate::bam::BinaryAsset,
        node_index: usize,
    ) {
        let character = match &asset.nodes[node_index] {
            PandaObject::Character(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        //TODO: make group node
        //TODO: apply node properties
        self.recurse_nodes(settings, context, asset, node_index);

        for bundle_ref in &character.node.bundle_refs {
            self.convert_character_bundle(settings, context, asset, *bundle_ref as usize, None);
        }
    }

    fn convert_character_bundle(
        &self, settings: &BamLoaderSettings, context: &mut LoadContext, asset: &crate::bam::BinaryAsset,
        node_index: usize, parent_joint: Option<&CharacterJoint>,
    ) {
        println!("{} {:?}", node_index, &asset.nodes[node_index]);
        match &asset.nodes[node_index] {
            PandaObject::CharacterJointBundle(node) => {
                for child_ref in &node.group.child_refs {
                    self.convert_character_bundle(settings, context, asset, *child_ref as usize, None);
                }
            }
            PandaObject::CharacterJoint(joint) => {
                // If this is a CharacterJoint, we actually need to process it, and process all children
                let mut default_value = joint.initial_net_transform_inverse.inverse();
                if let Some(parent_joint) = parent_joint {
                    if joint.initial_net_transform_inverse != parent_joint.initial_net_transform_inverse {
                        default_value *= parent_joint.initial_net_transform_inverse;
                    }
                }
                //inverse_bindposes.push(default_value), doesn't matter if it's not identity
                for child_ref in &joint.matrix.base.group.child_refs {
                    self.convert_character_bundle(settings, context, asset, *child_ref as usize, Some(joint));
                }
            }
            PandaObject::PartGroup(node) => {
                for child_ref in &node.child_refs {
                    self.convert_character_bundle(settings, context, asset, *child_ref as usize, None);
                }
            }
            _ => panic!("Something has gone horribly wrong!"),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BamLoaderSettings {
    /// If not empty, this will override material paths in the current
    pub material_path: String,
}

#[derive(Debug, Default)]
pub struct BamLoader;

impl AssetLoader for BamLoader {
    type Asset = BinaryAsset;
    type Error = bam::Error;
    type Settings = BamLoaderSettings;

    async fn load<'a>(
        &'a self, reader: &'a mut Reader<'_>, settings: &'a Self::Settings, context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        println!("ahhh {:?}", settings);
        // First, load the actual data into something we can pass around
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Then, parse the actual BAM file
        let bam = crate::bam::BinaryAsset::load(bytes)?;

        // Finally, we can actually generate the data
        let mut asset = BinaryAsset::default();
        asset.recurse_nodes(settings, context, &bam, 1);
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["bam"]
    }
}
