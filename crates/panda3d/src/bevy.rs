use std::path::PathBuf;

use bevy_animation::prelude::*;
use bevy_animation::{AnimationTarget, AnimationTargetId};
use bevy_app::prelude::*;
use bevy_asset::io::Reader;
use bevy_asset::prelude::*;
use bevy_asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy_color::prelude::*;
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;
use bevy_log::prelude::*;
use bevy_math::prelude::*;
use bevy_pbr::prelude::*;
use bevy_pbr::{ExtendedMaterial, MaterialExtension};
use bevy_reflect::prelude::*;
use bevy_render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_render::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy_render::prelude::*;
use bevy_render::render_asset::RenderAssetUsages;
use bevy_render::render_resource::*;
use bevy_render::texture::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy_scene::prelude::*;
use bevy_tasks::prelude::*;
use bevy_transform::prelude::*;
use bitflags::bitflags;
use hashbrown::HashMap;
use orthrus_core::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::nodes::color_attrib::ColorType;
use crate::nodes::cull_face_attrib::CullMode;
use crate::nodes::depth_write_attrib::DepthMode;
use crate::nodes::prelude::*;
use crate::nodes::sampler_state::{FilterType, WrapMode};
use crate::nodes::transform_state::TransformFlags;
use crate::nodes::transparency_attrib::TransparencyMode;
use crate::prelude::*;

//TODO: add node support, prepare collision, finish writing joint stuff, test animations, make it more robust
//by checking that we've actually visited all nodes?, also I think the character paths need to include the
//bundle, as well as all the tables

#[derive(Debug, Default)]
pub(crate) struct Effects {
    is_billboard: bool,
    is_decal: bool,
}

impl Effects {
    async fn new(asset: &BinaryAsset, parent: &Effects, render_effects: &RenderEffects) -> Self {
        let mut is_billboard = parent.is_billboard;
        let mut is_decal = parent.is_decal;

        for effect in &render_effects.effect_refs {
            match &asset.nodes[*effect as usize] {
                PandaObject::BillboardEffect(_) => {
                    //TODO: store the actual billboard data
                    is_billboard = true;
                }
                PandaObject::DecalEffect(_) => is_decal = true,
                PandaObject::CharacterJointEffect(_) => (), // We already handle Characters so just ignore it
                effect => panic!("Unexpected Render Effect! {:?}", effect),
            }
        }

        Self { is_billboard, is_decal }
    }
}

// Just steal this from bevy_gltf, it's a good structure
#[derive(Clone, Debug)]
struct AnimationContext {
    // The nearest ancestor animation root.
    root: Entity,
    // The path to the animation root. This is used for constructing the
    // animation target UUIDs.
    path: SmallVec<[Name; 8]>,
}

impl BinaryAsset {
    async fn handle_transform_state(
        &self, context: &mut LoadContext<'_>, transform_ref: usize,
    ) -> Result<SpatialBundle, bam::Error> {
        let transform_state = match &self.nodes[transform_ref] {
            PandaObject::TransformState(node) => node,
            _ => panic!("Unexpected TransformState node!"),
        };

        let bundle = if transform_state.flags.contains(TransformFlags::Identity) {
            // If we have an identity transform, just chuck the default in
            SpatialBundle::default()
        } else if transform_state.flags.contains(TransformFlags::MatrixKnown) {
            // We just have the raw matrix given to us, so construct a transform based off it
            SpatialBundle::from_transform(Transform::from_matrix(transform_state.matrix))
        } else if transform_state.flags.contains(TransformFlags::ComponentsGiven) {
            // We're given individual components, so let's piece together a transform
            if transform_state.rotation != Vec3::ZERO {
                warn!(
                    "Double check rotation on {:?}! Not sure if right axis.",
                    context.path()
                );
            }
            if transform_state.shear != Vec3::ZERO {
                warn!("Non-zero shear on {:?}! Ignoring, needs support.", context.path());
            }
            SpatialBundle::from_transform(Transform {
                translation: transform_state.position,
                rotation: match transform_state.flags.contains(TransformFlags::RotationGiven) {
                    true => Quat::from_euler(
                        EulerRot::YXZ,
                        transform_state.rotation.x,
                        transform_state.rotation.y,
                        transform_state.rotation.z,
                    ),
                    false => transform_state.quaternion,
                },
                scale: transform_state.scale,
            })
        } else {
            warn!("Unexpected TransformState data! See {:?}", context.path());
            SpatialBundle::default()
        };
        Ok(bundle)
    }

    /// This function is used to recursively convert all child nodes
    pub(crate) fn recurse_nodes(
        &self, world: &mut World, parent: Option<Entity>, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut PandaAsset, effects: &Effects, joint_data: Option<&SkinnedMesh>, node_index: usize,
    ) -> Result<(), bam::Error> {
        match &self.nodes[node_index] {
            PandaObject::ModelRoot(node) => {
                // If we've called this, we're at the scene root, create a named node and setup all children
                let entity = world.spawn((SpatialBundle::default(), Name::new(node.node.name.clone()))).id();
                if let Some(parent) = parent {
                    world.entity_mut(parent).add_child(entity);
                }

                let spatial_bundle =
                    block_on(self.handle_transform_state(context, node.node.transform_ref as usize))?;
                world.entity_mut(entity).insert(spatial_bundle);

                let render_effects = match &self.nodes[node.node.effects_ref as usize] {
                    PandaObject::RenderEffects(node) => node,
                    _ => panic!("Unexpected RenderEffects node!"),
                };

                let effects = block_on(Effects::new(&self, effects, render_effects));

                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        Some(entity),
                        settings,
                        context,
                        assets,
                        &effects,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::ModelNode(node) => {
                // We're either at the scene root, or an arbitrary child node, create a new node, process all
                // its attributes, and then recurse to any children.
                //println!("{} {} {:?}", node_index, node.node.name, node);
                let entity = world.spawn(Name::new(node.node.name.clone())).id();
                if let Some(parent) = parent {
                    world.entity_mut(parent).add_child(entity);
                }

                let spatial_bundle =
                    block_on(self.handle_transform_state(context, node.node.transform_ref as usize))?;
                world.entity_mut(entity).insert(spatial_bundle);

                let render_effects = match &self.nodes[node.node.effects_ref as usize] {
                    PandaObject::RenderEffects(node) => node,
                    _ => panic!("Unexpected RenderEffects node!"),
                };

                let effects = block_on(Effects::new(&self, effects, render_effects));

                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        Some(entity),
                        settings,
                        context,
                        assets,
                        &effects,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::PandaNode(node) => {
                // This is just used as a generic node, so spawn a new child and keep traversing
                let entity = world.spawn(Name::new(node.name.clone())).id();
                if let Some(parent) = parent {
                    world.entity_mut(parent).add_child(entity);
                }

                let spatial_bundle =
                    block_on(self.handle_transform_state(context, node.transform_ref as usize))?;
                world.entity_mut(entity).insert(spatial_bundle);

                let render_effects = match &self.nodes[node.effects_ref as usize] {
                    PandaObject::RenderEffects(node) => node,
                    _ => panic!("Unexpected RenderEffects node!"),
                };

                let effects = block_on(Effects::new(&self, effects, render_effects));

                for child_ref in &node.child_refs {
                    self.recurse_nodes(
                        world,
                        Some(entity),
                        settings,
                        context,
                        assets,
                        &effects,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::GeomNode(node) => {
                // This is considered a leaf node, which we process into a parent node with potentially
                // several Mesh+Material bundles. We might be getting called from a Character parent, so we
                // need to pass joint_data in. TODO: add a marker Component?
                let entity = world.spawn(Name::new(node.node.name.clone())).id();
                if let Some(parent) = parent {
                    world.entity_mut(parent).add_child(entity);
                }

                let spatial_bundle =
                    block_on(self.handle_transform_state(context, node.node.transform_ref as usize))?;
                world.entity_mut(entity).insert(spatial_bundle);

                let render_effects = match &self.nodes[node.node.effects_ref as usize] {
                    PandaObject::RenderEffects(node) => node,
                    _ => panic!("Unexpected RenderEffects node!"),
                };

                // First, let's create all the actual data
                block_on(self.convert_geom_node(
                    world, entity, settings, context, assets, node, &effects, joint_data,
                ))?;

                // In order to render properly, we should only apply effects (e.g. decals) to children it
                // seems? TODO: verify, otherwise we get weird clipping issues
                let effects = block_on(Effects::new(&self, effects, render_effects));

                // This may still have children, so handle those too
                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        Some(entity),
                        settings,
                        context,
                        assets,
                        &effects,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::Character(node) => {
                // This is a leaf node, which we process into a SkinnedMesh with both a Mesh+Material and a
                // bunch of Joint data. TODO: add a marker Component?
                let entity = world.spawn(Name::new(node.node.node.name.clone())).id();
                if let Some(parent) = parent {
                    world.entity_mut(parent).add_child(entity);
                }

                let spatial_bundle =
                    block_on(self.handle_transform_state(context, node.node.node.transform_ref as usize))?;
                world.entity_mut(entity).insert(spatial_bundle);

                let render_effects = match &self.nodes[node.node.node.effects_ref as usize] {
                    PandaObject::RenderEffects(node) => node,
                    _ => panic!("Unexpected RenderEffects node!"),
                };

                // First, let's handle all related CharacterBundles, and store the joint data for all child
                // geometry
                let joint_data = Some(block_on(
                    self.convert_character_node(world, entity, settings, context, assets, node_index),
                )?);

                let effects = block_on(Effects::new(&self, effects, render_effects));

                // Then, let's actually process those children
                for child_ref in &node.node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        Some(entity),
                        settings,
                        context,
                        assets,
                        &effects,
                        joint_data.as_ref(),
                        child_ref.0 as usize,
                    )?;
                }
            }
            PandaObject::AnimBundleNode(node) => {
                // This is considered a leaf node, which we'll process into an AnimationClip for the user to
                // render as needed. TODO: add a marker Component?
                let entity = world.spawn((SpatialBundle::default(), Name::new(node.node.name.clone()))).id();
                if let Some(parent) = parent {
                    world.entity_mut(parent).add_child(entity);
                }

                let render_effects = match &self.nodes[node.node.effects_ref as usize] {
                    PandaObject::RenderEffects(node) => node,
                    _ => panic!("Unexpected RenderEffects node!"),
                };

                // We just pass this through so we can treat it the same as convert_character_bundle
                self.convert_anim_node(world, entity, settings, context, assets, node)?;

                //TODO: any effects on AnimBundleNode?
                let effects = block_on(Effects::new(&self, effects, render_effects));

                // Then, let's actually process those children
                for child_ref in &node.node.child_refs {
                    self.recurse_nodes(
                        world,
                        Some(entity),
                        settings,
                        context,
                        assets,
                        &effects,
                        joint_data,
                        child_ref.0 as usize,
                    )?;
                }
            }
            //TODO: LODNode! this should still work with the highest LOD
            _ => (),
        }
        Ok(())
    }

    async fn convert_geom_node(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings, context: &mut LoadContext<'_>,
        assets: &mut PandaAsset, node: &GeomNode, effects: &Effects, joint_data: Option<&SkinnedMesh>,
    ) -> Result<(), bam::Error> {
        // Then let's create a new entity, and add our various properties.

        //TODO: handle tags, collide_mask?

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

            //TODO: if we have more than one primitive, we need to implement decomposition.
            assert!(geom.primitive_refs.len() == 1);
            //TODO: covering all our bases, I can support this but not right now
            assert!(geom.bounds_type == BoundsType::Default);
            self.convert_primitive(
                world,
                child,
                settings,
                context,
                assets,
                render_state,
                joint_data,
                geom.primitive_refs[0] as usize,
                geom.data_ref as usize,
                effects,
            )
            .await?;
        }

        Ok(())
    }

    async fn convert_primitive(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings,
        load_context: &mut LoadContext<'_>, assets: &mut PandaAsset, render_state: &RenderState,
        joint_data: Option<&SkinnedMesh>, node_index: usize, data_index: usize, effects: &Effects,
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

        // If we have a RenderState with attributes, we need to create a material
        let material = match render_state.attrib_refs.is_empty() {
            false => {
                let mut context = load_context.begin_labeled_asset();
                let label = format!("Material{}", assets.materials.len());
                let material = self.create_material(settings, &mut context, render_state, effects).await;
                let handle = load_context.add_loaded_labeled_asset(label, context.finish(material, None));
                assets.materials.push(handle.clone());
                handle
            }
            true => Handle::default(),
        };

        // We always have a Mesh to worry about, if this GeomPrimitive exists
        let context = load_context.begin_labeled_asset();
        let label = format!("Mesh{}", assets.meshes.len());
        let mesh_data = self
            .create_mesh(
                world,
                entity,
                primitive_node,
                primitive,
                vertex_data,
                vertex_format,
                joint_data,
            )
            .await;
        let mesh = load_context.add_loaded_labeled_asset(label, context.finish(mesh_data, None));
        assets.meshes.push(mesh.clone());

        world.entity_mut(entity).insert(MaterialMeshBundle::<Panda3DMaterial> {
            mesh,
            material,
            ..Default::default()
        });
        Ok(())
    }

    async fn create_material(
        &self, settings: &LoadSettings, context: &mut LoadContext<'_>, render_state: &RenderState,
        effects: &Effects,
    ) -> Panda3DMaterial {
        let mut material = Panda3DMaterial::default();

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
                            warn!("Encountered a TransparencyMode using multisamples, unimplemented!");
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
                        DepthMode::Off => false,
                        DepthMode::On => true,
                    };
                }
                PandaObject::CullBinAttrib(_attrib) => {
                    //TODO: this should already be done by bevy? might need to implement
                }
                _ => warn!("Unimplemented Attribute!"), //_ => panic!("Unknown RenderState attribute!"),
            }
        }

        if effects.is_decal {
            material.extension.decal_effect = true;
        }

        //TODO: create toggle when loading so users can choose to use actual lighting
        material.base.unlit = true;
        material.base.perceptual_roughness = 1.0;
        material.base.fog_enabled = false;

        //println!("Material: {:?} {:?}", material.base, material.extension);
        material
    }

    async fn create_mesh(
        &self, world: &mut World, _entity: Entity, primitive_node: &PandaObject, primitive: &GeomPrimitive,
        vertex_data: &GeomVertexData, vertex_format: &GeomVertexFormat, joint_data: Option<&SkinnedMesh>,
    ) -> Mesh {
        let topology = match primitive_node {
            PandaObject::GeomTristrips(_) => PrimitiveTopology::TriangleStrip,
            PandaObject::GeomTriangles(_) => PrimitiveTopology::TriangleList,
            _ => panic!("Unimplemented attribute!"),
        };
        //TODO: custom Mesh usages?
        let mut mesh = Mesh::new(topology, RenderAssetUsages::default());

        match primitive.vertices_ref {
            // If we have an associated ArrayData, then this polygon is indexed so we need to read it
            Some(vertex_ref) => {
                let array_data = match &self.nodes[vertex_ref as usize] {
                    PandaObject::GeomVertexArrayData(node) => node,
                    _ => panic!("Something has gone horribly wrong!"),
                };
                let array_format = match &self.nodes[array_data.array_format_ref as usize] {
                    PandaObject::GeomVertexArrayFormat(node) => node,
                    _ => panic!("Something has gone horribly wrong!"),
                };

                // This specific VertexArray should only ever have one specific entry: indices
                assert!(array_format.num_columns == 1);
                for column in &array_format.columns {
                    assert!(column.numeric_type == NumericType::U16);
                    let internal_name = match &self.nodes[column.name_ref as usize] {
                        PandaObject::InternalName(node) => node,
                        _ => panic!("Something has gone horribly wrong!"),
                    };
                    assert!(internal_name.name == "index");

                    let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);
                    let mut indices = Vec::with_capacity(data.len().unwrap() as usize / 2);
                    for _ in 0..indices.capacity() {
                        indices.push(data.read_u16().unwrap());
                    }
                    mesh.insert_indices(Indices::U16(indices));
                }
            }
            // Otherwise, we need to generate indices ourselves
            None => {
                let start = primitive.first_vertex as u32;
                let end = match primitive.num_vertices {
                    -1 => {
                        let ends = &self.arrays[primitive.ends_ref.unwrap() as usize];
                        assert!(ends.len() == 1);
                        ends[0] as u32
                    }
                    num_vertices => num_vertices as u32,
                };
                mesh.insert_indices(Indices::U32((start..start + end).collect()));
            }
        }

        // Now let's process the sub-array data. We always have at least one, so process that first
        let array_data = match &self.nodes[vertex_data.array_refs[0] as usize] {
            PandaObject::GeomVertexArrayData(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let array_format = match &self.nodes[vertex_format.array_refs[0] as usize] {
            PandaObject::GeomVertexArrayFormat(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let num_primitives: u64 = array_data.buffer.len() as u64 / u64::from(array_format.stride);
        let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);
        for column in &array_format.columns {
            // Use the InternalName as a lookup for what to do with each property
            let internal_name = match &self.nodes[column.name_ref as usize] {
                PandaObject::InternalName(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            //println!("{} {:?}", internal_name.name, column);

            match internal_name.name.as_str() {
                "vertex" => {
                    // Check our assumptions for this
                    assert!(column.num_components == 3);
                    assert!(column.numeric_type == NumericType::F32);
                    assert!(column.contents == Contents::Point);

                    // Then, build an array with the expected data
                    let mut vertex_data = Vec::with_capacity(num_primitives as usize);
                    for n in 0..num_primitives {
                        data.set_position(u64::from(column.start) + (u64::from(array_format.stride) * n))
                            .unwrap();

                        vertex_data.push([
                            data.read_f32().unwrap(),
                            data.read_f32().unwrap(),
                            data.read_f32().unwrap(),
                        ]);
                    }
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_data);
                }

                "normal" => {
                    // Check our assumptions for this
                    assert!(column.num_components == 3);
                    assert!(column.numeric_type == NumericType::F32);
                    assert!(column.contents == Contents::Vector || column.contents == Contents::Normal);

                    // Then, build an array with the expected data
                    let mut normal_data = Vec::with_capacity(num_primitives as usize);
                    for n in 0..num_primitives {
                        data.set_position(u64::from(column.start) + (u64::from(array_format.stride) * n))
                            .unwrap();

                        normal_data.push([
                            data.read_f32().unwrap(),
                            data.read_f32().unwrap(),
                            data.read_f32().unwrap(),
                        ]);
                    }
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal_data);
                }

                "tangent" => {
                    // Check our assumptions for this
                    assert!(column.num_components == 3);
                    assert!(column.numeric_type == NumericType::F32);
                    assert!(column.contents == Contents::Vector);

                    // Then, build an array with the expected data
                    let mut tangent_data = Vec::with_capacity(num_primitives as usize);
                    for n in 0..num_primitives {
                        data.set_position(u64::from(column.start) + (u64::from(array_format.stride) * n))
                            .unwrap();

                        // TODO: calculate handedness using the binormal column? For now, just set it to +1.0
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
                    // Check our assumptions for this
                    assert!(column.num_components == 3);
                    assert!(column.numeric_type == NumericType::F32);
                    assert!(column.contents == Contents::Vector);

                    // Then, build an array with the expected data. This isn't actually used outside of
                    // calculating the tangent handedness, so just skip any processing for now.
                    /*
                    let mut binormal_data = Vec::with_capacity(num_primitives);
                    for n in 0..num_primitives {
                        data.set_position(column.start as usize + (array_format.stride as usize * n));


                        binormal_data.push([
                            data.read_f32().unwrap(),
                            data.read_f32().unwrap(),
                            data.read_f32().unwrap(),
                        ]);
                    }
                    */
                }

                "texcoord" => {
                    // Check our assumptions for this
                    assert!(column.num_components == 2);
                    assert!(column.numeric_type == NumericType::F32);
                    assert!(column.contents == Contents::TexCoord);

                    // Then, build an array with the expected data
                    let mut texcoord_data = Vec::with_capacity(num_primitives as usize);
                    for n in 0..num_primitives {
                        data.set_position(u64::from(column.start) + (u64::from(array_format.stride) * n))
                            .unwrap();

                        // Panda3D stores flipped Y values to support OpenGL, so we do 1.0 - value.
                        texcoord_data.push([data.read_f32().unwrap(), 1.0 - data.read_f32().unwrap()]);
                    }
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, texcoord_data);
                }

                "color" => match column.numeric_type {
                    NumericType::PackedDABC => {
                        // Check our assumptions for this
                        assert!(column.num_components == 1);
                        assert!(column.contents == Contents::Color);

                        // Then, build an array with the expected data
                        let mut color_data = Vec::with_capacity(num_primitives as usize);
                        for n in 0..num_primitives {
                            data.set_position(u64::from(column.start) + (u64::from(array_format.stride) * n))
                                .unwrap();

                            let color = data.read_u32().unwrap();
                            let a = ((color >> 24) & 0xFF) as f32 / 255.0;
                            let r = ((color >> 16) & 0xFF) as f32 / 255.0;
                            let g = ((color >> 8) & 0xFF) as f32 / 255.0;
                            let b = ((color >> 0) & 0xFF) as f32 / 255.0;

                            // Bevy wants Linear RGBA vertex colors, so we need to do some conversion
                            color_data.push(Color::srgba(r, g, b, a).to_linear().to_vec4());
                        }
                        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, color_data);
                    }
                    _ => panic!("Unimplemented Color Column type!"),
                },

                _ => panic!("Unexpected Vertex Type! {} {:?}", internal_name.name, column),
            }
        }

        // If there's a second array referenced, we have a blend table to worry about.
        if vertex_data.array_refs.len() >= 2 {
            assert!(vertex_data.array_refs.len() == 2);

            let array_data = match &self.nodes[vertex_data.array_refs[1] as usize] {
                PandaObject::GeomVertexArrayData(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            let array_format = match &self.nodes[vertex_format.array_refs[1] as usize] {
                PandaObject::GeomVertexArrayFormat(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);

            for column in &array_format.columns {
                let internal_name = match &self.nodes[column.name_ref as usize] {
                    PandaObject::InternalName(node) => node,
                    _ => panic!("Something has gone horribly wrong!"),
                };
                //println!("{} {:?}", internal_name.name, column);

                match internal_name.name.as_str() {
                    "transform_blend" => {
                        let orig_blend_table =
                            match &self.nodes[vertex_data.transform_blend_table_ref.unwrap() as usize] {
                                PandaObject::TransformBlendTable(table) => table,
                                _ => panic!("Something has gone horribly wrong!"),
                            };

                        // We're first going to build a HashMap lookup with this BAM's ObjectID->Index, so we
                        // can take a shortcut when filling out the ATTRIBUTE_JOINT_WEIGHT and
                        // ATTRIBUTE_JOINT_INDEX.
                        //
                        // We have to walk the TransformBlendTable twice, but the number of joints is less
                        // than the number of blend combinations is less than the number of vertices, so this
                        // should overall save time.
                        let mut lookup = HashMap::new();
                        for transform in &orig_blend_table.blends {
                            for entry in &transform.entries {
                                if !lookup.contains_key(&entry.transform_ref) {
                                    // We've found a new joint, check what it's pointing to. This is a double
                                    // indirection which is rather expensive, but thankfully the number of
                                    // joints we have to check is comparatively tiny.
                                    let vertex_transform = match &self.nodes[entry.transform_ref as usize] {
                                        PandaObject::JointVertexTransform(node) => node,
                                        _ => panic!("Oops! Unimplemented VertexTransform node."),
                                    };
                                    let joint = match &self.nodes[vertex_transform.joint_ref as usize] {
                                        PandaObject::CharacterJoint(node) => node,
                                        _ => panic!("Oops! Unimplemented Joint type!"),
                                    };

                                    // Now that we have the joint, walk all entities til we find the same one
                                    let joint_data = &joint_data.unwrap().joints;
                                    // I don't think there's any avoiding walking all entities until we find
                                    // the correct joint name but again, the number should be pretty tiny.
                                    for joint_id in 0..joint_data.len() {
                                        let entity = joint_data[joint_id];
                                        if **world.entity(entity).get::<Name>().unwrap()
                                            == *joint.matrix.base.group.name
                                        {
                                            // Now, this specific transform_ref can be translated directly to
                                            // a joint index for Mesh::ATTRIBUTE_JOINT_INDEX
                                            lookup.insert(entry.transform_ref, joint_id as u16);
                                        }
                                    }
                                }
                            }
                        }

                        // In the spirit of making things fast, let's build a table for the actual weights
                        // we're writing since the number of TransformBlends is way less then the number of
                        // primitives (usually)

                        let mut transforms: Vec<([u16; 4], [f32; 4])> =
                            Vec::with_capacity(orig_blend_table.blends.len());
                        for transform in &orig_blend_table.blends {
                            // Clone it and sort it, generate our blend data for this specific blend
                            let mut sorted = transform.entries.clone();
                            sorted.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
                            let cleaned = sorted.iter().take(4).collect::<Vec<_>>();
                            let mut indices = [0u16, 0u16, 0u16, 0u16];
                            let mut weights = [0f32, 0f32, 0f32, 0f32];
                            //println!("{:?}", sorted);
                            for n in 0..cleaned.len() {
                                indices[n] = lookup[&cleaned[n].transform_ref];
                                weights[n] = cleaned[n].weight;
                            }

                            // Normalize it if we need to
                            let net_weight: f32 = weights.iter().sum();
                            if net_weight != 0.0 {
                                for weight in weights.iter_mut() {
                                    *weight /= net_weight;
                                }
                            }
                            transforms.push((indices, weights));
                        }

                        // Now, let's actually build ATTRIBUTE_JOINT_WEIGHT and ATTRIBUTE_JOINT_INDEX
                        let mut blend_lookup = vec![[0u16, 0u16, 0u16, 0u16]; num_primitives as usize];
                        let mut blend_table = vec![[0f32, 0f32, 0f32, 0f32]; num_primitives as usize];
                        for n in 0..num_primitives {
                            // First, verify we're good to read the lookup table
                            assert!(column.numeric_type == NumericType::U16);
                            assert!(column.contents == Contents::Index);
                            data.set_position(u64::from(column.start) + (u64::from(array_format.stride) * n))
                                .unwrap();

                            // Then, let's write the blend data
                            let lookup_id = data.read_u16().unwrap();
                            blend_lookup[n as usize] = transforms[lookup_id as usize].0;
                            blend_table[n as usize] = transforms[lookup_id as usize].1;
                        }

                        // In order to correctly render, the SkinnedMesh needs to be on this specific entity,
                        // so add it now that we know we need it.
                        /*mesh.insert_attribute(
                            Mesh::ATTRIBUTE_JOINT_INDEX,
                            VertexAttributeValues::Uint16x4(blend_lookup),
                        );*/
                        //mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, blend_table);
                        //world.entity_mut(entity).insert(joint_data.unwrap().clone());
                    }

                    _ => panic!("Unexpected Vertex Type! {} {:?}", internal_name.name, column),
                }
            }
        } /* else {
              // If we've been passed joint data, but the mesh doesn't actually have any, we still want to
              // parent it to be animated
              if let Some(joint_data) = joint_data {
                  mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0]; num_primitives]));
                  mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1f32, 0f32, 0f32, 0f32]; num_primitives]);
                  world.entity_mut(entity).insert(joint_data.clone());
              }
          }*/

        //println!("Mesh: {:?}", mesh);
        mesh
    }

    async fn convert_character_node(
        &self, world: &mut World, entity: Entity, settings: &LoadSettings, context: &mut LoadContext<'_>,
        assets: &mut PandaAsset, node_index: usize,
    ) -> Result<SkinnedMesh, bam::Error> {
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
        assert!(character.node.bundle_refs.len() == 1);
        let (child_inverse_bindposes, child_joints) = self.convert_character_bundle(
            world,
            entity,
            settings,
            context,
            assets,
            character.node.bundle_refs[0] as usize,
            None,
            None,
            None,
        )?;
        inverse_bindposes.extend(child_inverse_bindposes);
        joints.extend(child_joints);

        //println!("Bindposes: {:?}", inverse_bindposes);
        //println!("Joints: {:?}", joints);

        // Add the Bindpose to our list of assets and get its handle
        let label = format!("Bindpose{}", assets.bindposes.len());
        let inverse_bindposes =
            context.labeled_asset_scope(label, |_| SkinnedMeshInverseBindposes::from(inverse_bindposes));
        assets.bindposes.push(inverse_bindposes.clone()); //Cloning a handle is cheap, thankfully

        // The SkinnedMesh needs to be attached to specific primitives so we can't handle it here, just send
        // it back

        Ok(SkinnedMesh { inverse_bindposes, joints })
    }

    fn convert_character_bundle(
        &self, world: &mut World, parent: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut PandaAsset, node_index: usize, mut animation_context: Option<AnimationContext>,
        root_transform: Option<Mat4>, parent_joint: Option<&CharacterJoint>,
    ) -> Result<(Vec<Mat4>, Vec<Entity>), bam::Error> {
        let mut inverse_bindposes = Vec::new();
        let mut joints = Vec::new();
        match &self.nodes[node_index] {
            PandaObject::CharacterJointBundle(node) => {
                //TODO: this needs work
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
                        parent_joint,
                    )?;
                    inverse_bindposes.extend(child_inverse_bindposes);
                    joints.extend(child_joints);
                }
            }
            PandaObject::CharacterJoint(joint) => {
                // We have an actual joint, so we need to compute the inverse bindpose and create a new node
                // with a Transform
                /*let net_transform = match parent_joint {
                    Some(parent_joint) => {
                        joint.matrix.value * parent_joint.initial_net_transform_inverse.inverse()
                    }
                    None => joint.matrix.value * root_transform.unwrap(),
                };
                let _skinning_matrix = joint.initial_net_transform_inverse * net_transform;*/
                let default_value = match parent_joint {
                    Some(parent_joint) => {
                        joint.initial_net_transform_inverse.inverse()
                            * parent_joint.initial_net_transform_inverse
                    }
                    None => joint.initial_net_transform_inverse.inverse(),
                };

                println!(
                    "{} Default Value {}\n{}\n{}\n{}",
                    joint.matrix.base.group.name,
                    joint.matrix.default_value.x_axis,
                    joint.matrix.default_value.y_axis,
                    joint.matrix.default_value.z_axis,
                    joint.matrix.default_value.w_axis
                );

                // Create a new entity
                let name = Name::new(joint.matrix.base.group.name.clone());
                let joint_entity = world
                    .spawn((
                        TransformBundle::from(Transform::from_matrix(default_value)),
                        name.clone(),
                    ))
                    .id();
                world.entity_mut(parent).add_child(joint_entity);

                if let Some(animation_context) = animation_context.as_mut() {
                    animation_context.path.push(name);
                    println!("{:?}", animation_context);
                    world.entity_mut(joint_entity).insert(AnimationTarget {
                        id: AnimationTargetId::from_names(animation_context.path.iter()),
                        player: animation_context.root,
                    });
                }

                inverse_bindposes.push(joint.initial_net_transform_inverse);
                joints.push(joint_entity);

                for child_ref in &joint.matrix.base.group.child_refs {
                    let (child_inverse_bindposes, child_joints) = self.convert_character_bundle(
                        world,
                        joint_entity,
                        settings,
                        context,
                        assets,
                        *child_ref as usize,
                        animation_context.clone(),
                        root_transform,
                        Some(joint),
                    )?;
                    inverse_bindposes.extend(child_inverse_bindposes);
                    joints.extend(child_joints);
                }

                if let Some(animation_context) = animation_context.as_mut() {
                    animation_context.path.pop();
                }
            }
            PandaObject::PartGroup(node) => {
                // If we run into a plain PartGroup, let's treat it as the beginning of the skeleton, so we
                // can map animations to it easier. This is analogous to running into an AnimGroup when
                // building an animation. Grab the child and operate off it.

                // First, grab the child joint so we can set up the root node
                assert!(node.child_refs.len() == 1);
                let joint = match &self.nodes[node.child_refs[0] as usize] {
                    PandaObject::CharacterJoint(node) => node,
                    node => panic!("Unexpected PartGroup child! {:?}", node),
                };

                // Create an entity for it
                let name = Name::from(joint.matrix.base.group.name.clone());
                let skeleton = world
                    .spawn((
                        AnimationPlayer::default(),
                        TransformBundle::from(Transform::from_matrix(Mat4::default())),
                        name.clone(),
                    ))
                    .id();
                // Save the root Entity so we can pull it later
                assets.anim_players.push(skeleton);

                // We know this is the root, so create a new animation context to keep track of everything
                let mut animation_context = AnimationContext { root: skeleton, path: SmallVec::new() };
                animation_context.path.push(name);
                println!("{:?}", animation_context);
                world.entity_mut(skeleton).insert(AnimationTarget {
                    id: AnimationTargetId::from_names(animation_context.path.iter()),
                    player: animation_context.root,
                });

                println!(
                    "{} Default Value {}\n{}\n{}\n{}",
                    joint.matrix.base.group.name,
                    joint.matrix.default_value.x_axis,
                    joint.matrix.default_value.y_axis,
                    joint.matrix.default_value.z_axis,
                    joint.matrix.default_value.w_axis
                );

                inverse_bindposes.push(Mat4::default());
                joints.push(skeleton);
                world.entity_mut(parent).add_child(skeleton);
                for child_ref in &joint.matrix.base.group.child_refs {
                    let (child_inverse_bindposes, child_joints) = self.convert_character_bundle(
                        world,
                        skeleton,
                        settings,
                        context,
                        assets,
                        *child_ref as usize,
                        Some(animation_context.clone()),
                        root_transform,
                        Some(joint),
                    )?;
                    inverse_bindposes.extend(child_inverse_bindposes);
                    joints.extend(child_joints);
                }
            }
            _ => panic!("Something has gone horribly wrong!"),
        }
        Ok((inverse_bindposes, joints))
    }

    fn convert_anim_node(
        &self, world: &mut World, parent: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut PandaAsset, node: &AnimBundleNode,
    ) -> Result<(), bam::Error> {
        // We're at an AnimBundleNode, let's create an AnimationClip, grab our AnimBundle, and manually handle
        // skeletal and mesh animation data separately.
        let bundle = match &self.nodes[node.anim_bundle_ref as usize] {
            PandaObject::AnimBundle(node) => node,
            _ => panic!("Unexpected AnimBundleNode!"),
        };

        let mut animation_clip = AnimationClip::default();

        assert!(bundle.group.child_refs.len() == 2);
        let skeleton_group = match &self.nodes[bundle.group.child_refs[0] as usize] {
            PandaObject::AnimGroup(node) => node,
            _ => panic!("Unexpected AnimGroup node!"),
        };
        if skeleton_group.child_refs.len() == 1 {
            //TODO: this will probably break

            self.convert_transform_table(
                world,
                parent,
                settings,
                context,
                assets,
                skeleton_group.child_refs[0] as usize,
                &mut animation_clip,
                None,
                bundle.fps,
                bundle.num_frames as usize,
            )?;
        }

        let _morph_group = match &self.nodes[bundle.group.child_refs[1] as usize] {
            PandaObject::AnimGroup(node) => node,
            _ => panic!("Unexpected AnimGroup node!"),
        };
        assert!(_morph_group.child_refs.len() == 0);
        //TODO: actually handle morph data? I don't currently.

        //println!("{:?}", animation_clip);

        let label = format!("Animation{}", assets.animations.len());
        context.labeled_asset_scope(label, |_| animation_clip);

        Ok(())
    }

    fn convert_transform_table(
        &self, world: &mut World, parent: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut PandaAsset, node_index: usize, animation_clip: &mut AnimationClip,
        mut animation_context: Option<AnimationContext>, fps: f32, num_frames: usize,
    ) -> Result<(), bam::Error> {
        match &self.nodes[node_index] {
            PandaObject::AnimChannelMatrixXfmTable(node) => {
                // These joints don't actually "exist", they only encode information, so we only create the
                // Name to generate the AnimationContext so we can grab an AnimationTargetId as need be
                let name = Name::from(node.matrix.group.name.clone());

                // Create the AnimationContext if this is the root, and then push the current name
                if animation_context.is_none() {
                    animation_context = Some(AnimationContext { root: parent, path: SmallVec::new() });
                }

                if let Some(ref mut animation_context) = animation_context {
                    animation_context.path.push(name.clone());

                    // Now let's create the AnimationTargetId
                    let anim_target_id = AnimationTargetId::from_names(animation_context.path.iter());

                    /*for n in 0..12 {
                        println!(
                            "{} {} {} {} {:?}",
                            fps,
                            num_frames,
                            "ijkabcrphxyz".chars().nth(n).unwrap(),
                            node.tables[n].len(),
                            node.tables[n]
                        );
                    }*/

                    // Handle each "group" of transforms
                    for n in 0..4 {
                        let node0 = &node.tables[n * 3 + 0];
                        let node1 = &node.tables[n * 3 + 1];
                        let node2 = &node.tables[n * 3 + 2];
                        if !node0.is_empty() || !node1.is_empty() || !node2.is_empty() {
                            if n == 1 {
                                warn!("Unable to handle animations with shear! {:?}", context.path());
                                continue; // Let's skip it, try not to panic
                            }

                            // Get the default value for what table it is
                            let default = match n {
                                0 => 1.0, // Scale
                                2 => 0.0, // Rotation
                                3 => 0.0, // Translation
                                _ => unreachable!(),
                            };

                            // Now we need to generate full tables for the number of frames there is. If it's
                            // empty, generate the default value, if it's one, extend that value over the
                            // whole range, otherwise use the full data.
                            let node0 = match node0.len() {
                                0 => &vec![default; num_frames],
                                1 => &vec![node0[0]; num_frames],
                                _ => node0,
                            };

                            let node1 = match node1.len() {
                                0 => &vec![default; num_frames],
                                1 => &vec![node1[0]; num_frames],
                                _ => node1,
                            };

                            let node2 = match node2.len() {
                                0 => &vec![default; num_frames],
                                1 => &vec![node2[0]; num_frames],
                                _ => node2,
                            };

                            //println!("{} {} {}", node0.len(), node1.len(), node2.len());

                            let keyframes = match n {
                                0 => {
                                    let mut aggregate = Vec::with_capacity(num_frames);
                                    for index in 0..num_frames {
                                        aggregate.push(Vec3::new(
                                            node0[index].to_radians(),
                                            node1[index].to_radians(),
                                            node2[index].to_radians(),
                                        ));
                                    }
                                    Keyframes::Scale(aggregate)
                                }
                                2 => {
                                    let mut aggregate = Vec::with_capacity(num_frames);
                                    for index in 0..num_frames {
                                        aggregate.push(Quat::from_euler(
                                            EulerRot::ZXY,
                                            node0[index].to_radians(),
                                            node1[index].to_radians(),
                                            node2[index].to_radians(),
                                        ));
                                    }
                                    Keyframes::Rotation(aggregate)
                                }
                                3 => {
                                    let mut aggregate = Vec::with_capacity(num_frames);
                                    for index in 0..num_frames {
                                        aggregate.push(Vec3::new(
                                            node0[index].to_radians(),
                                            node1[index].to_radians(),
                                            node2[index].to_radians(),
                                        ));
                                    }
                                    Keyframes::Translation(aggregate)
                                }
                                _ => unreachable!(),
                            };

                            //println!("{} {:?}", (0..num_frames).len(), keyframes);

                            animation_clip.add_curve_to_target(
                                anim_target_id,
                                VariableCurve {
                                    keyframe_timestamps: (0..num_frames).map(|i| i as f32 / fps).collect(),
                                    keyframes,
                                    interpolation: Interpolation::Linear,
                                },
                            );
                        }
                    }

                    for child_ref in &node.matrix.group.child_refs {
                        self.convert_transform_table(
                            world,
                            parent,
                            settings,
                            context,
                            assets,
                            *child_ref as usize,
                            animation_clip,
                            Some(animation_context.clone()),
                            fps,
                            num_frames,
                        )?;
                    }
                }
            }
            node => panic!("Unexpected TransformTable node! {:?}", node),
        }
        Ok(())
    }

    /*fn convert_animGroup_node(
        &self, world: &mut World, parent: Entity, settings: &LoadSettings, context: &mut LoadContext,
        assets: &mut PandaAsset, node_index: usize, mut animation_context: Option<AnimationContext>,
    ) -> Result<(), bam::Error> {
        match &self.nodes[node_index] {
            PandaObject::AnimBundleNode(node) => {
                for child_ref in &node.group.child_refs {
                    self.convert_animGroup_node(
                        world,
                        parent,
                        settings,
                        context,
                        assets,
                        *child_ref as usize,
                        None,
                    )?;
                }
            }
            PandaObject::CharacterJoint(joint) => {
                // We have an actual joint, so we need to compute the inverse bindpose and create a new node
                // with a Transform
                /*let net_transform = match parent_joint {
                    Some(parent_joint) => {
                        joint.matrix.value * parent_joint.initial_net_transform_inverse.inverse()
                    }
                    None => joint.matrix.value * root_transform.unwrap(),
                };
                let _skinning_matrix = joint.initial_net_transform_inverse * net_transform;*/

                // Create a new entity
                let name = Name::new(joint.matrix.base.group.name.clone());
                let joint_entity = world
                    .spawn((
                        TransformBundle::from(Transform::from_matrix(joint.matrix.default_value)),
                        name.clone(),
                    ))
                    .id();
                world.entity_mut(parent).add_child(joint_entity);

                if let Some(animation_context) = animation_context.as_mut() {
                    animation_context.path.push(name);
                    world.entity_mut(joint_entity).insert(AnimationTarget {
                        id: AnimationTargetId::from_names(animation_context.path.iter()),
                        player: animation_context.root,
                    });
                }

                for child_ref in &joint.matrix.base.group.child_refs {
                    self.convert_anim_bundle(
                        world,
                        joint_entity,
                        settings,
                        context,
                        assets,
                        *child_ref as usize,
                        animation_context.clone(),
                    )?;
                }

                if let Some(animation_context) = animation_context.as_mut() {
                    animation_context.path.pop();
                }
            }
            PandaObject::PartGroup(node) => {
                // If we run into a plain PartGroup, let's treat it as the beginning of the skeleton, so we
                // can map animations to it easier. This is analogous to running into an AnimGroup when
                // building an animation. Grab the child and operate off it.

                // First, grab the child joint so we can set up the root node
                assert!(node.child_refs.len() == 1);
                let joint = match &self.nodes[node.child_refs[0] as usize] {
                    PandaObject::CharacterJoint(node) => node,
                    node => panic!("Unexpected PartGroup child! {:?}", node),
                };

                // Create an entity for it
                let name = Name::from(joint.matrix.base.group.name.clone());
                let skeleton = world
                    .spawn((
                        AnimationPlayer::default(),
                        TransformBundle::from(Transform::from_matrix(joint.matrix.default_value)),
                        name.clone(),
                    ))
                    .id();

                // We know this is the root, so create a new animation context to keep track of everything
                let mut animation_context = AnimationContext { root: skeleton, path: SmallVec::new() };
                animation_context.path.push(name);
                world.entity_mut(skeleton).insert(AnimationTarget {
                    id: AnimationTargetId::from_names(animation_context.path.iter()),
                    player: animation_context.root,
                });

                world.entity_mut(parent).add_child(skeleton);
                for child_ref in &joint.matrix.base.group.child_refs {
                    self.convert_anim_bundle(
                        world,
                        skeleton,
                        settings,
                        context,
                        assets,
                        *child_ref as usize,
                        Some(animation_context.clone()),
                    )?;
                }
            }
            _ => panic!("Something has gone horribly wrong!"),
        }
        Ok(())
    }*/
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoadSettings {
    /// If not empty, this will override material paths in the current
    pub material_path: String,
}

#[derive(Debug, Default)]
pub struct BamLoader;

#[derive(Asset, TypePath, Debug, Default)]
pub struct PandaAsset {
    pub scenes: Vec<Handle<Scene>>,
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<Panda3DMaterial>>,
    pub bindposes: Vec<Handle<SkinnedMeshInverseBindposes>>,
    pub anim_players: Vec<Entity>,
    pub animations: Vec<Handle<AnimationClip>>,
}

impl AssetLoader for BamLoader {
    type Asset = PandaAsset;
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
        let mut assets = PandaAsset::default();
        bam.recurse_nodes(
            &mut world,
            None,
            settings,
            load_context,
            &mut assets,
            &Effects::default(),
            None,
            1,
        )?;
        let scene = load_context.labeled_asset_scope("Scene0".to_string(), |_| Scene::new(world));
        assets.scenes.push(scene);
        info!(
            "Model {:?} loaded in {:?}",
            load_context.asset_path(),
            start_time.elapsed()
        );

        Ok(assets)
    }

    fn extensions(&self) -> &[&str] {
        &["bam"]
    }
}

pub struct Panda3DPlugin;

impl Plugin for Panda3DPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        //load_internal_asset!(app, SHADER_HANDLE, "shader.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, Panda3DExtension>,
        >::default())
            .init_asset_loader::<BamLoader>()
            .init_asset::<PandaAsset>();
    }
}

//const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(63068044759449526956158328048017573322);

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[bind_group_data(Panda3DExtensionKey)]
pub struct Panda3DExtension {
    depth_write_enabled: bool,
    decal_effect: bool,
}

impl Default for Panda3DExtension {
    fn default() -> Self {
        Self { depth_write_enabled: true, decal_effect: false }
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
                }
            }
            match key.bind_group_data.contains(Panda3DExtensionKey::DECAL_EFFECT) {
                true => {
                    depth_stencil.bias.constant = 1; //TODO: tweak these more if they give any trouble
                    depth_stencil.bias.slope_scale = 0.5;
                    depth_stencil.depth_write_enabled = false;
                }
                false => (),
            }
        }
        Ok(())
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Panda3DExtensionKey: u64 {
        const DEPTH_WRITE_ENABLED = 0x80000000;
        const DECAL_EFFECT = 0x100000000;
    }
}

impl From<&Panda3DExtension> for Panda3DExtensionKey {
    fn from(extension: &Panda3DExtension) -> Self {
        let mut key = Panda3DExtensionKey::empty();
        key.set(
            Panda3DExtensionKey::DEPTH_WRITE_ENABLED,
            extension.depth_write_enabled,
        );
        key.set(Panda3DExtensionKey::DECAL_EFFECT, extension.decal_effect);
        key
    }
}

pub type Panda3DMaterial = ExtendedMaterial<StandardMaterial, Panda3DExtension>;
