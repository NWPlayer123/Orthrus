use std::path::PathBuf;

use crate::bam;
use crate::nodes::prelude::*;
use crate::prelude::*;

use bevy_asset::io::Reader;
use bevy_asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy_core::Name;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use bevy_hierarchy::BuildWorldChildren;
use bevy_log::prelude::*;
use bevy_pbr::StandardMaterial;
use bevy_render::alpha::AlphaMode;
use bevy_render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy_render::render_asset::RenderAssetUsages;
use bevy_scene::Scene;
use orthrus_core::prelude::*;
use serde::{Deserialize, Serialize};

impl BinaryAsset {
    /// This function is used to recursively convert all child nodes
    pub(crate) fn recurse_nodes(
        &self, world: &mut World, parent: Entity, settings: &LoadSettings, context: &mut LoadContext,
        node_index: usize, joint_data: Option<&Vec<u32>>,
    ) -> Result<(), bam::Error> {
        match &self.nodes[node_index] {
            PandaObject::ModelRoot(node) => {
                // If we've called this, we're at the scene root, create a named node and setup all children
                let child = world.spawn(Name::new(node.node.name.clone())).id();
                world.entity_mut(parent).add_child(child);

                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(world, child, settings, context, child_ref.0 as usize, joint_data)?;
                }
            }
            PandaObject::ModelNode(node) => {
                // If we've called this, we're either at the scene root or an arbitrary child node, so just
                // create a named node and setup children
                let child = world.spawn(Name::new(node.node.name.clone())).id();
                world.entity_mut(parent).add_child(child);

                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(world, child, settings, context, child_ref.0 as usize, joint_data)?;
                }
            }
            PandaObject::PandaNode(node) => {
                // This is just used as a generic node, so spawn a new child and keep traversing
                let child = world.spawn(Name::new(node.name.clone())).id();
                world.entity_mut(parent).add_child(child);

                for child_ref in &node.child_refs {
                    self.recurse_nodes(world, child, settings, context, child_ref.0 as usize, joint_data)?;
                }
            }
            PandaObject::GeomNode(node) => {
                // This is considered a leaf node, so create a single entity and spawn all data off of this
                let entity = world.spawn(Name::new(node.node.name.clone())).id();
                world.entity_mut(parent).add_child(entity);

                // First, let's create all the actual data
                self.convert_geom_node(world, entity, settings, context, node_index, joint_data)?;

                // This may still have children, so handle those
                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(world, entity, settings, context, child_ref.0 as usize, joint_data)?;
                }
            }
            PandaObject::Character(node) => {
                // This is considered a leaf node, so create a single entity and spawn all data off of this
                let entity = world.spawn(Name::new(node.node.node.name.clone())).id();
                world.entity_mut(parent).add_child(entity);

                // First, let's handle all related CharacterBundles, and store the joint data for all child geometry
                let joint_data =
                    Some(self.convert_character_node(world, entity, settings, context, node_index)?);

                // Then, let's actually process those children
                for child_ref in &node.node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        entity,
                        settings,
                        context,
                        child_ref.0 as usize,
                        joint_data.as_ref(),
                    )?;
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn convert_geom_node(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings, context: &mut LoadContext,
        node_index: usize, joint_data: Option<&Vec<u32>>,
    ) -> Result<(), bam::Error> {
        // First, let's actually grab the node so we can access all its properties
        let node = match &self.nodes[node_index] {
            PandaObject::GeomNode(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };

        // TODO: Then, handle any node properties like billboard, transformstate, renderstate whenever they come up

        // Finally, let's process all the actual geometry
        for geom_ref in &node.geom_refs {
            // Get the relevant nodes
            let geom = match &self.nodes[geom_ref.0 as usize] {
                PandaObject::Geom(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            let render_state = match &self.nodes[geom_ref.1 as usize] {
                PandaObject::RenderState(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };

            // Then, process all primitives
            for primitive_ref in &geom.primitive_refs {
                // TODO: get node and decompose it? We also should pass vertex_data
                // Also TODO: reorder variables? Not sure they make much sense this way
                self.convert_primitive(
                    world,
                    entity,
                    settings,
                    context,
                    render_state,
                    geom.data_ref as usize,
                    *primitive_ref as usize,
                    joint_data,
                )?;
            }
        }

        Ok(())
    }

    fn convert_primitive(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings, context: &mut LoadContext,
        render_state: &RenderState, data_index: usize, node_index: usize, joint_data: Option<&Vec<u32>>,
    ) -> Result<(), bam::Error> {
        // First, load the GeomPrimitive and all the associated GeomVertex indices data
        let primitive_node = &self.nodes[node_index];
        let primitive = match primitive_node {
            PandaObject::GeomTristrips(node) => node,
            PandaObject::GeomTriangles(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let vertex_data = match &self.nodes[data_index] {
            PandaObject::GeomVertexData(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let vertex_format = match &self.nodes[vertex_data.format_ref as usize] {
            PandaObject::GeomVertexFormat(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let array_data = match &self.nodes[primitive.vertices_ref.unwrap() as usize] {
            PandaObject::GeomVertexArrayData(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let array_format = match &self.nodes[array_data.array_format_ref as usize] {
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
            let label = format!("Material0"); //TODO: use world/child.id()
            let _material_handle = context.labeled_asset_scope(label, |context| {
                let mut material = StandardMaterial::default();
                material.unlit = true; //TODO: disable this? Toontown specific

                for attrib_ref in &render_state.attrib_refs {
                    println!(
                        "Render State {} {:?}",
                        attrib_ref.0, &self.nodes[attrib_ref.0 as usize]
                    );
                    match &self.nodes[attrib_ref.0 as usize] {
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
                                let texture_stage = match &self.nodes[stage.texture_stage_ref as usize] {
                                    PandaObject::TextureStage(node) => node,
                                    _ => panic!("Something has gone horribly wrong!"),
                                };
                                let texture = match &self.nodes[stage.texture_ref as usize] {
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

                println!("Material: {:?}", material);
                material
            });
        }

        let _num_primitives = match primitive_node {
            PandaObject::GeomTristrips(node) => {
                // This is a complex primitive, so we need to load ends_ref and get the length of the array
                self.arrays[node.ends_ref.unwrap() as usize].len()
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

        let label = format!("Mesh0");
        let _mesh_handle = context.labeled_asset_scope(label, |_context| {
            let topology = match primitive_node {
                PandaObject::GeomTristrips(_) => PrimitiveTopology::TriangleStrip,
                PandaObject::GeomTriangles(_) => PrimitiveTopology::TriangleList,
                _ => panic!("Unimplemented attribute!"),
            };
            //TODO: custom Mesh usages?
            let mut mesh = Mesh::new(topology, RenderAssetUsages::default());

            //First, let's process the GeomVertex data we got above, which should always just be a list of indices
            assert!(array_format.num_columns == 1);
            let internal_name = match &self.nodes[array_format.columns[0].name_ref as usize] {
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
                let array_data = match &self.nodes[vertex_data.array_refs[n] as usize] {
                    PandaObject::GeomVertexArrayData(node) => node,
                    _ => panic!("Something has gone horribly wrong!"),
                };
                let array_format = match &self.nodes[vertex_format.array_refs[n] as usize] {
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
                    let internal_name = match &self.nodes[column.name_ref as usize] {
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
                                texcoord_data
                                    .push([data.read_f32().unwrap(), 1.0 - data.read_f32().unwrap()]);
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

            println!("Mesh: {:?}", mesh);
            mesh
        });
        Ok(())
    }

    fn convert_character_node(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings, context: &mut LoadContext,
        node_index: usize,
    ) -> Result<Vec<u32>, bam::Error> {
        let character = match &self.nodes[node_index] {
            PandaObject::Character(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        //TODO: make group node
        //TODO: apply node properties

        for bundle_ref in &character.node.bundle_refs {
            self.convert_character_bundle(settings, context, *bundle_ref as usize, None)?;
        }
        Ok(Vec::new())
    }

    fn convert_character_bundle(
        &self, settings: &LoadSettings, context: &mut LoadContext, node_index: usize,
        parent_joint: Option<&CharacterJoint>,
    ) -> Result<(), bam::Error> {
        println!("{} {:?}", node_index, &self.nodes[node_index]);
        match &self.nodes[node_index] {
            PandaObject::CharacterJointBundle(node) => {
                for child_ref in &node.group.child_refs {
                    self.convert_character_bundle(settings, context, *child_ref as usize, None)?;
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
                    self.convert_character_bundle(settings, context, *child_ref as usize, Some(joint))?;
                }
            }
            PandaObject::PartGroup(node) => {
                for child_ref in &node.child_refs {
                    self.convert_character_bundle(settings, context, *child_ref as usize, None)?;
                }
            }
            _ => panic!("Something has gone horribly wrong!"),
        }
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoadSettings {
    /// If not empty, this will override material paths in the current
    pub material_path: String,
}

#[derive(Debug, Default)]
pub struct BamLoader;

impl AssetLoader for BamLoader {
    type Asset = Scene;
    type Error = bam::Error;
    type Settings = LoadSettings;

    async fn load<'a>(
        &'a self, reader: &'a mut Reader<'_>, settings: &'a Self::Settings, context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // First, load the actual data into something we can pass around
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Then, parse the actual BAM file
        let asset = crate::bam::BinaryAsset::load(bytes)?;

        // Finally, we can actually generate the data
        let mut world = World::default();
        let root_entity = world.spawn(()).id();
        asset.recurse_nodes(&mut world, root_entity, settings, context, 1, None)?;

        Ok(Scene::new(world))
    }

    fn extensions(&self) -> &[&str] {
        &["bam"]
    }
}
