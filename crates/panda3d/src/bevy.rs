use std::path::PathBuf;

use bevy_app::Plugin;
use bevy_asset::io::Reader;
use bevy_asset::prelude::*;
use bevy_asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy_color::prelude::*;
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;
use bevy_log::prelude::*;
use bevy_pbr::prelude::*;
use bevy_pbr::{ExtendedMaterial, MaterialExtension};
use bevy_reflect::prelude::*;
use bevy_render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_render::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy_render::prelude::*;
use bevy_render::render_asset::RenderAssetUsages;
use bevy_render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy_render::texture::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy_scene::prelude::*;
use bevy_transform::prelude::*;
use bitflags::bitflags;
use orthrus_core::prelude::*;
use serde::{Deserialize, Serialize};

use crate::nodes::prelude::*;
use crate::prelude::*;

//TODO: add node support, prepare collision, finish writing joint stuff, test animations

impl BinaryAsset {
    /// This function is used to recursively convert all child nodes
    pub(crate) fn recurse_nodes(
        &self, world: &mut World, parent: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut BamAssets, joint_data: Option<&Vec<Entity>>, node_index: usize,
    ) -> Result<(), bam::Error> {
        match &self.nodes[node_index] {
            PandaObject::ModelRoot(node) => {
                // If we've called this, we're at the scene root, create a named node and setup all children
                let child = world.spawn((SpatialBundle::default(), Name::new(node.node.name.clone()))).id();
                world.entity_mut(parent).add_child(child);

                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        child,
                        settings,
                        context,
                        assets,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::ModelNode(node) => {
                // If we've called this, we're either at the scene root or an arbitrary child node, so just
                // create a named node and setup children
                let child = world.spawn((SpatialBundle::default(), Name::new(node.node.name.clone()))).id();
                world.entity_mut(parent).add_child(child);

                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        child,
                        settings,
                        context,
                        assets,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::PandaNode(node) => {
                // This is just used as a generic node, so spawn a new child and keep traversing
                let child = world.spawn((SpatialBundle::default(), Name::new(node.name.clone()))).id();
                world.entity_mut(parent).add_child(child);

                for child_ref in &node.child_refs {
                    self.recurse_nodes(
                        world,
                        child,
                        settings,
                        context,
                        assets,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::GeomNode(node) => {
                // This is considered a leaf node, so create a single entity and spawn all data off of this
                let entity = world.spawn((SpatialBundle::default(), Name::new(node.node.name.clone()))).id();
                world.entity_mut(parent).add_child(entity);

                // First, let's create all the actual data
                self.convert_geom_node(world, entity, settings, context, assets, joint_data, node_index)?;

                // This may still have children, so handle those
                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        entity,
                        settings,
                        context,
                        assets,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::Character(node) => {
                // This is considered a leaf node, so create a single entity and spawn all data off of this
                let entity =
                    world.spawn((SpatialBundle::default(), Name::new(node.node.node.name.clone()))).id();
                world.entity_mut(parent).add_child(entity);

                // First, let's handle all related CharacterBundles, and store the joint data for all child
                // geometry
                let joint_data =
                    Some(self.convert_character_node(world, entity, settings, context, assets, node_index)?);

                // Then, let's actually process those children
                for child_ref in &node.node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        entity,
                        settings,
                        context,
                        assets,
                        joint_data.as_ref(),
                        child_ref.0 as usize,
                    )?;
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn convert_geom_node(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut BamAssets, joint_data: Option<&Vec<Entity>>, node_index: usize,
    ) -> Result<(), bam::Error> {
        // First, let's actually grab the node so we can access all its properties
        let node = match &self.nodes[node_index] {
            PandaObject::GeomNode(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };

        //println!("{}", node.node.name);

        // TODO: Then, handle any node properties like billboard, transformstate, renderstate whenever they
        // come up

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

            // Each Entity can only render one PbrBundle, so we have to create separate entities for each Geom
            let child = world.spawn(()).id();
            world.entity_mut(entity).add_child(child);

            // Then, process all primitives
            for primitive_ref in &geom.primitive_refs {
                // TODO: get node and decompose it? We also should pass vertex_data
                // Also TODO: reorder variables? Not sure they make much sense this way
                self.convert_primitive(
                    world,
                    child,
                    settings,
                    context,
                    assets,
                    render_state,
                    joint_data,
                    *primitive_ref as usize,
                    geom.data_ref as usize,
                )?;
            }
        }

        Ok(())
    }

    fn convert_primitive(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut BamAssets, render_state: &RenderState, _joint_data: Option<&Vec<Entity>>,
        node_index: usize, data_index: usize,
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
        let mut array_data = None;
        let mut array_format = None;
        if let Some(vertex_ref) = primitive.vertices_ref {
            array_data = match &self.nodes[vertex_ref as usize] {
                PandaObject::GeomVertexArrayData(node) => Some(node),
                _ => panic!("Something has gone horribly wrong!"),
            };
            array_format = match &self.nodes[array_data.unwrap().array_format_ref as usize] {
                PandaObject::GeomVertexArrayFormat(node) => Some(node),
                _ => panic!("Something has gone horribly wrong!"),
            };
        }

        // If we have a RenderState with attributes, we need to create a material
        let material = match render_state.attrib_refs.is_empty() {
            false => {
                let label = format!("Material{}", assets.materials.len());
                let material_handle = context.labeled_asset_scope(label, |context| {
                    let mut material = Panda3DMaterial::default();
                    material.base.unlit = true; //TODO: disable this? Toontown specific

                    for attrib_ref in &render_state.attrib_refs {
                        //println!("{} {:?}", attrib_ref.0, &self.nodes[attrib_ref.0 as usize]);
                        match &self.nodes[attrib_ref.0 as usize] {
                            PandaObject::TransparencyAttrib(attrib) => {
                                material.base.alpha_mode = match attrib.mode {
                                    TransparencyMode::None => AlphaMode::Opaque,
                                    TransparencyMode::Alpha => AlphaMode::Blend,
                                    TransparencyMode::PremultipliedAlpha => AlphaMode::Premultiplied,
                                    TransparencyMode::Binary => AlphaMode::Mask(0.5),
                                    // TODO: bevy needs better support for OIT transparency
                                    // See: https://github.com/bevyengine/bevy/issues/2223
                                    TransparencyMode::Dual => AlphaMode::AlphaToCoverage,
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
                                    if !settings.material_path.is_empty() {
                                        let mut new_path = PathBuf::from(settings.material_path.clone());
                                        new_path.push(image_path.file_name().unwrap());
                                        image_path = new_path;
                                    }
                                    image_path.set_extension("png");

                                    // Apply sampler properties to the loaded image
                                    let address_mode_u = match texture.body.default_sampler.wrap_u {
                                        WrapMode::Clamp => ImageAddressMode::ClampToEdge,
                                        WrapMode::Repeat => ImageAddressMode::Repeat,
                                        WrapMode::Mirror => ImageAddressMode::MirrorRepeat,
                                        _ => panic!("Unimplemented Texture WrapMode!"),
                                    };
                                    let address_mode_v = match texture.body.default_sampler.wrap_v {
                                        WrapMode::Clamp => ImageAddressMode::ClampToEdge,
                                        WrapMode::Repeat => ImageAddressMode::Repeat,
                                        WrapMode::Mirror => ImageAddressMode::MirrorRepeat,
                                        _ => panic!("Unimplemented Texture WrapMode!"),
                                    };
                                    let address_mode_w = match texture.body.default_sampler.wrap_w {
                                        WrapMode::Clamp => ImageAddressMode::ClampToEdge,
                                        WrapMode::Repeat => ImageAddressMode::Repeat,
                                        WrapMode::Mirror => ImageAddressMode::MirrorRepeat,
                                        _ => panic!("Unimplemented Texture WrapMode!"),
                                    };
                                    let mag_filter = match texture.body.default_sampler.mag_filter {
                                        FilterType::Nearest => ImageFilterMode::Nearest,
                                        FilterType::Linear => ImageFilterMode::Linear,
                                        _ => panic!("Unimplemented mag filter type!"),
                                    };
                                    let min_filter = match texture.body.default_sampler.min_filter {
                                        FilterType::Nearest => ImageFilterMode::Nearest,
                                        FilterType::Linear => ImageFilterMode::Linear,
                                        FilterType::NearestMipmapNearest => ImageFilterMode::Nearest,
                                        FilterType::LinearMipmapNearest => ImageFilterMode::Linear,
                                        FilterType::NearestMipmapLinear => ImageFilterMode::Nearest,
                                        FilterType::LinearMipmapLinear => ImageFilterMode::Linear,
                                        _ => panic!("Unimplemented min filter type!"),
                                    };
                                    let mipmap_filter = match texture.body.default_sampler.min_filter {
                                        FilterType::Nearest => ImageFilterMode::Nearest,
                                        FilterType::Linear => ImageFilterMode::Linear,
                                        FilterType::NearestMipmapNearest => ImageFilterMode::Nearest,
                                        FilterType::LinearMipmapNearest => ImageFilterMode::Nearest,
                                        FilterType::NearestMipmapLinear => ImageFilterMode::Linear,
                                        FilterType::LinearMipmapLinear => ImageFilterMode::Linear,
                                        _ => panic!("Unimplemented mipmap filter type!"),
                                    };
                                    let image = context
                                        .loader()
                                        .with_settings(move |s: &mut _| {
                                            *s = ImageLoaderSettings {
                                                sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                                                    address_mode_u,
                                                    address_mode_v,
                                                    address_mode_w,
                                                    mag_filter,
                                                    min_filter,
                                                    mipmap_filter,
                                                    ..Default::default()
                                                }),
                                                ..Default::default()
                                            }
                                        })
                                        .load(image_path);
                                    material.base.base_color_texture = Some(image);
                                }
                            }
                            PandaObject::CullFaceAttrib(attrib) => {
                                material.base.cull_mode = match attrib.mode {
                                    CullMode::None => None,
                                    CullMode::Clockwise => Some(Face::Front),
                                    CullMode::CounterClockwise => Some(Face::Back),
                                    CullMode::Unchanged => {
                                        warn!("CullMode Unchanged! Unsure what to do with this.");
                                        Some(Face::Back)
                                    }
                                };
                            }
                            PandaObject::ColorAttrib(attrib) => {
                                material.base.base_color = match attrib.color_type {
                                    ColorType::Flat => Color::Srgba(Srgba::from_vec4(attrib.color)),
                                    ColorType::Vertex => {
                                        if attrib.color != Vec4::ZERO {
                                            warn!("Vertex Color not zero! Unimplemented");
                                        }
                                        Color::WHITE
                                    }
                                    ColorType::Off => {
                                        warn!("Vertex Color off, unimplemented!");
                                        Color::WHITE
                                    }
                                };
                            }
                            PandaObject::DepthWriteAttrib(attrib) => {
                                material.extension.depth_write_enabled = match attrib.mode {
                                    DepthMode::Off => 0,
                                    DepthMode::On => 1,
                                };
                            }
                            PandaObject::CullBinAttrib(_attrib) => {
                                //TODO: this should already be done by bevy? might need to implement
                            }
                            _ => warn!("Unimplemented Attribute!"), //_ => panic!("Unknown RenderState attribute!"),
                        }
                    }

                    //println!("Material: {:?} {:?}", material.base, material.extension);
                    material
                });
                assets.materials.push(material_handle.clone());
                material_handle
            }
            true => Handle::default(),
        };

        let label = format!("Mesh{}", assets.meshes.len());
        let mesh = context.labeled_asset_scope(label, |_context| {
            let topology = match primitive_node {
                PandaObject::GeomTristrips(_) => PrimitiveTopology::TriangleStrip,
                PandaObject::GeomTriangles(_) => PrimitiveTopology::TriangleList,
                _ => panic!("Unimplemented attribute!"),
            };
            //TODO: custom Mesh usages?
            let mut mesh = Mesh::new(topology, RenderAssetUsages::default());

            if let Some(array_data) = array_data {
                let array_format = array_format.unwrap();
                //First, let's process the GeomVertex data we got above, which should always just be a list
                // of indices
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
            }

            // Now process all sub-array data. If there's more than one array, we're using a blend table of
            // some sort
            for n in 0..vertex_data.array_refs.len() {
                let array_data = match &self.nodes[vertex_data.array_refs[n] as usize] {
                    PandaObject::GeomVertexArrayData(node) => node,
                    _ => panic!("Something has gone horribly wrong!"),
                };
                let array_format = match &self.nodes[vertex_format.array_refs[n] as usize] {
                    PandaObject::GeomVertexArrayFormat(node) => node,
                    _ => panic!("Something has gone horribly wrong!"),
                };
                let num_primitives = array_data.buffer.len() / array_format.stride as usize;
                let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);
                for column in &array_format.columns {
                    // For each sub-array, process all columns individually
                    let internal_name = match &self.nodes[column.name_ref as usize] {
                        PandaObject::InternalName(node) => node,
                        _ => panic!("Something has gone horribly wrong!"),
                    };

                    //println!("{} {:?}", internal_name.name, column);

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
                        "normal" => {
                            assert!(column.num_components == 3);
                            assert!(column.contents == Contents::Vector);
                            assert!(column.numeric_type == NumericType::F32);
                            let mut normal_data = Vec::with_capacity(num_primitives);
                            for n in 0..num_primitives {
                                data.set_position(column.start as usize + (array_format.stride as usize * n));
                                normal_data.push([
                                    data.read_f32().unwrap(),
                                    data.read_f32().unwrap(),
                                    data.read_f32().unwrap(),
                                ]);
                            }
                            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal_data);
                        }
                        "tangent" => {
                            assert!(column.num_components == 3);
                            assert!(column.contents == Contents::Vector);
                            assert!(column.numeric_type == NumericType::F32);
                            let mut tangent_data = Vec::with_capacity(num_primitives);
                            for n in 0..num_primitives {
                                data.set_position(column.start as usize + (array_format.stride as usize * n));
                                tangent_data.push([
                                    data.read_f32().unwrap(),
                                    data.read_f32().unwrap(),
                                    data.read_f32().unwrap(),
                                    1.0,
                                ]);
                            }
                            mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangent_data);
                        }
                        "binormal" => {
                            assert!(column.num_components == 3);
                            assert!(column.contents == Contents::Vector);
                            assert!(column.numeric_type == NumericType::F32);
                            let mut binormal_data = Vec::with_capacity(num_primitives);
                            for n in 0..num_primitives {
                                data.set_position(column.start as usize + (array_format.stride as usize * n));
                                binormal_data.push([
                                    data.read_f32().unwrap(),
                                    data.read_f32().unwrap(),
                                    data.read_f32().unwrap(),
                                ]);
                            }
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
                        "color" => match column.numeric_type {
                            NumericType::PackedDABC => {
                                let mut color_data = Vec::with_capacity(num_primitives);
                                for n in 0..num_primitives {
                                    data.set_position(
                                        column.start as usize + (array_format.stride as usize * n),
                                    );
                                    let color = data.read_u32().unwrap();
                                    let a = ((color >> 24) & 0xFF) as f32 / 255.0;
                                    let r = ((color >> 16) & 0xFF) as f32 / 255.0;
                                    let g = ((color >> 8) & 0xFF) as f32 / 255.0;
                                    let b = ((color >> 0) & 0xFF) as f32 / 255.0;
                                    color_data.push(Color::srgba(r, g, b, a).to_linear().to_vec4());
                                }
                                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, color_data);
                            }
                            _ => panic!("Unimplemented Color Column type!"),
                        },
                        "transform_blend" => {
                            //TODO: refactoring
                        }
                        _ => panic!("Unimplemented Vertex Type! {} {:?}", internal_name.name, column),
                    }
                }
            }

            if mesh.indices().is_none() {
                let ends = &self.arrays[primitive.ends_ref.unwrap() as usize];
                assert!(ends.len() == 1);
                let indices: Vec<u32> =
                    (primitive.first_vertex as u32..primitive.first_vertex as u32 + ends[0]).collect();
                // We need to use primitive.first_vertex
                mesh.insert_indices(Indices::U32(indices));
            }

            //println!("Mesh: {:?}", mesh);
            mesh
        });
        assets.meshes.push(mesh.clone());
        world.entity_mut(entity).insert(MaterialMeshBundle::<Panda3DMaterial> {
            mesh,
            material,
            ..Default::default()
        });
        Ok(())
    }

    fn convert_character_node(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut BamAssets, node_index: usize,
    ) -> Result<Vec<Entity>, bam::Error> {
        let character = match &self.nodes[node_index] {
            PandaObject::Character(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        //TODO: make group node
        //TODO: apply node properties

        // Collect all Bindpose and Joint data so we can create a SkinnedMesh
        let mut inverse_bindposes = Vec::new();
        let mut joints = Vec::new();

        // Iterate all children in the bundle to actually create the hierarchy
        for bundle_ref in &character.node.bundle_refs {
            let (child_inverse_bindposes, child_joints) = self.convert_character_bundle(
                world,
                entity,
                settings,
                context,
                assets,
                *bundle_ref as usize,
                None,
                None,
            )?;
            inverse_bindposes.extend(child_inverse_bindposes);
            joints.extend(child_joints);
        }

        //println!("Bindposes: {:?}", inverse_bindposes);
        //println!("Joints: {:?}", joints);

        // Add the Bindpose to our list of assets and get its handle
        let label = format!("Bindpose{}", assets.bindposes.len());
        let inverse_bindpose_handle =
            context.labeled_asset_scope(label, |_| SkinnedMeshInverseBindposes::from(inverse_bindposes));
        assets.bindposes.push(inverse_bindpose_handle.clone()); //Cloning a handle is cheap, thankfully

        // Create the actual SkinnedMesh for this entity
        world
            .entity_mut(entity)
            .insert(SkinnedMesh { inverse_bindposes: inverse_bindpose_handle, joints: joints.clone() });

        Ok(joints)
    }

    fn convert_character_bundle(
        &self, world: &mut World, parent: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut BamAssets, node_index: usize, parent_joint: Option<&CharacterJoint>,
        root_transform: Option<Mat4>,
    ) -> Result<(Vec<Mat4>, Vec<Entity>), bam::Error> {
        let mut inverse_bindposes = Vec::new();
        let mut joints = Vec::new();
        match &self.nodes[node_index] {
            PandaObject::CharacterJointBundle(node) => {
                for child_ref in &node.group.child_refs {
                    let (child_inverse_bindposes, child_joints) = self.convert_character_bundle(
                        world,
                        parent,
                        settings,
                        context,
                        assets,
                        *child_ref as usize,
                        None,
                        Some(node.root_transform),
                    )?;
                    inverse_bindposes.extend(child_inverse_bindposes);
                    joints.extend(child_joints);
                }
            }
            PandaObject::CharacterJoint(joint) => {
                // We have an actual joint, so we need to compute the inverse bindpose and create a new node
                // with a Transform
                let net_transform = match parent_joint {
                    Some(parent_joint) => {
                        joint.matrix.value * parent_joint.initial_net_transform_inverse.inverse()
                    }
                    None => joint.matrix.value * root_transform.unwrap(),
                };
                let skinning_matrix = joint.initial_net_transform_inverse * net_transform;

                // Create a new entity, attach a TransformBundle and Name
                let joint_entity = world
                    .spawn((
                        TransformBundle::from(Transform::from_matrix(joint.matrix.default_value)),
                        Name::new(joint.matrix.base.group.name.clone()),
                    ))
                    .id();
                world.entity_mut(parent).add_child(joint_entity);

                inverse_bindposes.push(skinning_matrix);
                joints.push(joint_entity);

                for child_ref in &joint.matrix.base.group.child_refs {
                    let (child_inverse_bindposes, child_joints) = self.convert_character_bundle(
                        world,
                        joint_entity,
                        settings,
                        context,
                        assets,
                        *child_ref as usize,
                        Some(joint),
                        root_transform,
                    )?;
                    inverse_bindposes.extend(child_inverse_bindposes);
                    joints.extend(child_joints);
                }
            }
            PandaObject::PartGroup(node) => {
                for child_ref in &node.child_refs {
                    let (child_inverse_bindposes, child_joints) = self.convert_character_bundle(
                        world,
                        parent,
                        settings,
                        context,
                        assets,
                        *child_ref as usize,
                        None,
                        root_transform,
                    )?;
                    inverse_bindposes.extend(child_inverse_bindposes);
                    joints.extend(child_joints);
                }
            }
            _ => panic!("Something has gone horribly wrong!"),
        }
        Ok((inverse_bindposes, joints))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoadSettings {
    /// If not empty, this will override material paths in the current
    pub material_path: String,
}

#[derive(Debug, Default)]
pub struct BamLoader;

#[derive(Debug, Default)]
pub struct BamAssets {
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<Panda3DMaterial>>,
    pub bindposes: Vec<Handle<SkinnedMeshInverseBindposes>>,
}

impl AssetLoader for BamLoader {
    type Asset = Scene;
    type Error = bam::Error;
    type Settings = LoadSettings;

    async fn load<'a>(
        &'a self, reader: &'a mut Reader<'_>, settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let start_time = std::time::Instant::now();
        // First, load the actual data into something we can pass around
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Then, parse the actual BAM file
        let bam = crate::bam::BinaryAsset::load(bytes)?;

        // Finally, we can actually generate the data
        let mut world = World::default();
        let root_entity = world.spawn(SpatialBundle::default()).id();
        let mut assets = BamAssets::default();
        bam.recurse_nodes(
            &mut world,
            root_entity,
            settings,
            load_context,
            &mut assets,
            None,
            1,
        )?;
        let scene = Scene::new(world);
        println!("Model loaded in {:?}", start_time.elapsed());
        Ok(scene)
    }

    fn extensions(&self) -> &[&str] {
        &["bam"]
    }
}

pub struct Panda3DPlugin;

impl Plugin for Panda3DPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, Panda3DExtension>,
        >::default())
            .init_asset_loader::<BamLoader>();
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(Panda3DExtensionKey)]
pub struct Panda3DExtension {
    #[uniform(100)]
    depth_write_enabled: u32,
}

impl Default for Panda3DExtension {
    fn default() -> Self {
        Self { depth_write_enabled: 1 }
    }
}

impl MaterialExtension for Panda3DExtension {
    fn specialize(
        _pipeline: &bevy_pbr::MaterialExtensionPipeline, descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef, key: bevy_pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            match key.bind_group_data.contains(Panda3DExtensionKey::DEPTH_WRITE_ENABLED) {
                true => {
                    depth_stencil.depth_write_enabled = true;
                }
                false => {
                    depth_stencil.depth_write_enabled = false;
                    //depth_stencil.depth_compare = CompareFunction::Always; TODO?
                }
            }
        }
        Ok(())
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Panda3DExtensionKey: u64 {
        const DEPTH_WRITE_ENABLED = 0x80000000;
    }
}

impl From<&Panda3DExtension> for Panda3DExtensionKey {
    fn from(extension: &Panda3DExtension) -> Self {
        let mut key = Panda3DExtensionKey::empty();
        key.set(
            Panda3DExtensionKey::DEPTH_WRITE_ENABLED,
            extension.depth_write_enabled != 0,
        );
        key
    }
}

pub type Panda3DMaterial = ExtendedMaterial<StandardMaterial, Panda3DExtension>;
