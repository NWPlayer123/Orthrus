use std::collections::BTreeMap;

use bevy_internal::animation::{AnimationTarget, AnimationTargetId};
/// This file is designed to provide loading Panda3D BAM assets into the Bevy game engine. This obviously
/// comes with some complexities, as Panda3D and specifically Toontown's scene graph minutia are poorly
/// documented.
///
/// For example, all Toontown models begin with a ModelRoot or ModelNode that serves as the root node of
/// the .egg file they were converted from. Additionally, specific nodes serve specific purposes. A
/// Character node is designed to be a high level animatable node that multiple meshes attach to, as well
/// as a singular (TODO: check) PartBundle that holds all skinning data
use bevy_internal::asset::io::Reader;
use bevy_internal::asset::{AssetLoader, LoadContext};
use bevy_internal::image::{ImageAddressMode, ImageFilterMode, ImageSamplerBorderColor};
use bevy_internal::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy_internal::prelude::*;
use bevy_internal::render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_internal::render::mesh::MeshVertexBufferLayoutRef;
use bevy_internal::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureFormat,
};
use bevy_internal::tasks::block_on;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use crate::bevy_sgi::SgiImageLoader;
use crate::nodes::color_attrib::ColorType;
use crate::nodes::cull_face_attrib::CullMode;
use crate::nodes::dispatch::NodeRef;
use crate::nodes::part_bundle::BlendType;
use crate::nodes::prelude::*;
use crate::nodes::sampler_state::{FilterType, WrapMode};
use crate::nodes::transform_state::TransformFlags;
use crate::nodes::transparency_attrib::TransparencyMode;
//use crate::bevy::Effects;
use crate::prelude::*;

// TODO on this whole file, try to reduce nesting, should be able to create an internal Error type, return
// result and error if we encounter unexpected data, instead of the current stupid if let Some() spam.

#[derive(Debug, Default, Clone, Copy)]
struct Effects {
    is_billboard: bool,
    is_decal: bool,
}

impl Effects {
    async fn new(assets: &BinaryAsset, parent: Option<&Effects>, node_index: usize) -> Self {
        let mut result = match parent {
            Some(effects) => *effects,
            None => Self::default(),
        };

        let Some(effects) = assets.nodes.get_as::<RenderEffects>(node_index) else {
            warn!(name: "not_a_render_effects", target: "Panda3DLoader",
                "Tried to access node {}, but it's not a RenderEffects, ignoring.", node_index);
            return result;
        };

        for effect in &effects.effect_refs {
            match assets.nodes.get(*effect as usize) {
                Some(node) => match node {
                    // TODO: actually handle billboards
                    NodeRef::BillboardEffect(_) => result.is_billboard = true,
                    NodeRef::DecalEffect(_) => result.is_decal = true,
                    // We handle Characters separately, TODO verify that this isn't needed using our new
                    // setup
                    NodeRef::CharacterJointEffect(_) => {}
                    _ => {
                        warn!(name: "unknown_render_effect", target: "Panda3DLoader",
                            "Unknown RenderEffects: node {}, ignoring.", effect)
                    }
                },
                None => {
                    warn!(name: "unexpected_node_index", target: "Panda3DLoader",
                        "Tried to access node {}, but it doesn't exist, ignoring.", effect)
                }
            }
        }

        result
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
    async fn recurse_nodes(
        &self, loader: &mut AssetLoaderData<'_, '_>, parent: Option<Entity>, effects: Option<&Effects>,
        joint_data: Option<&SkinnedMesh>, net_nodes: Option<&BTreeMap<usize, Entity>>, node_index: usize,
    ) {
        match self.nodes.get(node_index) {
            Some(NodeRef::ModelNode(node)) => {
                // This can either be a ModelNode or a ModelRoot, either way we need to spawn a new node to
                // attach stuff to.
                let (entity, effects) =
                    self.handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index).await;

                // TODO: handle transform: Local correctly?
                if node.attributes != 0 {
                    warn!(name: "model_node_attribs_unhandled", target: "Panda3DLoader",
                        "ModelNode {} has attributes attached that we don't handle, please fix!", node_index);
                }

                for child_ref in &node.child_refs {
                    if child_ref.1 != 0 {
                        warn!(name: "nonzero_node_sort", target: "Panda3DLoader",
                            "Node {} has a child with non-zero sort order, please fix!", node_index);
                    }
                    Box::pin(self.recurse_nodes(
                        loader,
                        Some(entity),
                        Some(&effects),
                        joint_data,
                        net_nodes,
                        child_ref.0 as usize,
                    ))
                    .await;
                }
            }
            Some(NodeRef::PandaNode(node)) => {
                // This is just a plain ol' node, so just process its data and explore all children.
                let (entity, effects) =
                    self.handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index).await;

                for child_ref in &node.child_refs {
                    if child_ref.1 != 0 {
                        warn!(name: "nonzero_node_sort", target: "Panda3DLoader",
                            "Node {} has a child with non-zero sort order, please fix!", node_index);
                    }
                    Box::pin(self.recurse_nodes(
                        loader,
                        Some(entity),
                        Some(&effects),
                        joint_data,
                        net_nodes,
                        child_ref.0 as usize,
                    ))
                    .await;
                }
            }
            Some(NodeRef::Character(node)) => {
                // Characters are helper nodes that group together multiple meshes together with
                // animation data. TODO: add a marker Component?
                let (entity, effects) =
                    self.handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index).await;

                // TODO: figure out if this ever happens
                if node.bundle_refs.len() != 1 {
                    warn!(name: "unexpected_character_node", target: "Panda3DLoader",
                        "Character Node {} has more than one associated CharacterJointBundle, ignoring.", node_index);
                }

                // First, let's process the `CharacterJointBundle` into [`SkinnedMesh`] data, as well as any
                // net nodes we spawned to add an [`AnimationTarget`]. TODO: make a
                // non-recursive function to simplify this mess?
                let mut net_nodes = BTreeMap::new();
                let (inverse_bindposes, joints) = self
                    .convert_joint_bundle(
                        loader.world,
                        entity,
                        None,
                        &mut net_nodes,
                        node.bundle_refs[0] as usize,
                    )
                    .await;

                // TODO: migrate to bevy_gltf's new enum-based system so this is less dumb
                let label = format!("Bindpose{}", loader.assets.bindposes.len());
                let inverse_bindposes = loader
                    .context
                    .labeled_asset_scope(label, |_| SkinnedMeshInverseBindposes::from(inverse_bindposes));
                loader.assets.bindposes.push(inverse_bindposes.clone());
                // We need to attach this to any children Entities we spawn for them to have the correct mesh
                // joint data.
                let skinned_mesh = SkinnedMesh { inverse_bindposes, joints };

                // Then, we need to process all child nodes
                for child_ref in &node.child_refs {
                    if child_ref.1 != 0 {
                        warn!(name: "nonzero_node_sort", target: "Panda3DLoader",
                            "Node {} has a child with non-zero sort order, please fix!", node_index);
                    }
                    Box::pin(self.recurse_nodes(
                        loader,
                        Some(entity),
                        Some(&effects),
                        Some(&skinned_mesh),
                        Some(&net_nodes),
                        child_ref.0 as usize,
                    ))
                    .await;
                }
            }
            Some(NodeRef::GeomNode(node)) => {
                // We need to create and attach actual mesh data to this node.
                let (entity, effects) =
                    self.handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index).await;

                //TODO handle tags, collide_mask?

                for geom_ref in &node.geom_refs {
                    self.convert_geom_node(loader, geom_ref.0 as usize, geom_ref.1 as usize, entity).await;
                }

                // Then, we need to process all child nodes
                for child_ref in &node.child_refs {
                    if child_ref.1 != 0 {
                        warn!(name: "nonzero_node_sort", target: "Panda3DLoader",
                            "Node {} has a child with non-zero sort order, please fix!", node_index);
                    }
                    Box::pin(self.recurse_nodes(
                        loader,
                        Some(entity),
                        Some(&effects),
                        joint_data,
                        net_nodes,
                        child_ref.0 as usize,
                    ))
                    .await;
                }
            }
            Some(node) => println!("Unexpected node {:?} in recurse_nodes", node),
            None => {
                warn!(name: "unexpected_node_index", target: "Panda3DLoader",
                    "Tried to access node {}, but it doesn't exist, ignoring.", node_index);
            }
        }
    }

    /// Constructs a [`Transform`] from a given `TransformState`. Used for any node that inherits from
    /// `PandaNode`.
    async fn handle_transform_state(&self, node_index: usize) -> Transform {
        if let Some(node) = self.nodes.get_as::<TransformState>(node_index) {
            if node.flags.contains(TransformFlags::Identity) {
                Transform::default()
            } else if node.flags.contains(TransformFlags::MatrixKnown) {
                Transform::from_matrix(node.matrix)
            } else if node.flags.contains(TransformFlags::ComponentsGiven) {
                // Components are given separately, so we need to construct a transform from them.
                let translation = node.position;
                let rotation = match node.flags.contains(TransformFlags::QuaternionGiven) {
                    true => node.quaternion,
                    false => {
                        // Yaw/Pitch/Roll
                        Quat::from_euler(EulerRot::YXZ, node.rotation.x, node.rotation.y, node.rotation.z)
                    }
                };
                let scale = node.scale;
                if node.shear != Vec3::ZERO {
                    warn!(name: "shear_transform_unimplemented", target: "Panda3DLoader",
                        "Detected a non-zero shear on node {}, which is currently unsupported, ignoring.", node_index);
                }
                Transform::from_translation(translation).with_rotation(rotation).with_scale(scale)
            } else {
                warn!(name: "unexpected_transform_state", target: "Panda3DLoader",
                    "Potentially malformed TransformState: node {}, ignoring.", node_index);
                Transform::default()
            }
        } else {
            warn!(name: "not_a_transform_state", target: "Panda3DLoader",
                "Tried to access node {}, but it's not a TransformState, ignoring.", node_index);
            Transform::default()
        }
    }

    /// Handles all data relevant to `PandaNode` entities, and spawns a new object into the world.
    async fn handle_panda_node(
        &self, world: &mut World, parent: Option<Entity>, effects: Option<&Effects>,
        net_nodes: Option<&BTreeMap<usize, Entity>>, node: &PandaNode, node_index: usize,
    ) -> (Entity, Effects) {
        // TODO: We don't current handle RenderState, for now, grab it and check if it's empty
        if let Some(render_state) = self.nodes.get_as::<RenderState>(node.state_ref as usize) {
            if !render_state.attrib_refs.is_empty() {
                warn!(name: "unhandled_render_state", target: "Panda3DLoader",
                    "Non-empty RenderState attached to node {} being ignored! Please fix.", node_index);
            }
        } else {
            warn!(name: "not_a_render_state", target: "Panda3DLoader",
                "Tried to access node {}, but it's not a RenderState, ignoring.", node.state_ref);
        }

        // Handle our Transform so we can spawn a new entity
        let transform = self.handle_transform_state(node.transform_ref as usize).await;

        // We only see what data is attached to a RenderEffects so we can pass it down to child nodes, TODO:
        // figure out proper inheritance
        let effects = Effects::new(self, effects, node.effects_ref as usize).await;

        // Check all of the parameters I've been ignoring, warn if any of them aren't the default, TODO
        if node.draw_control_mask != 0
            || node.draw_show_mask != 0xFFFFFFFF
            || node.into_collide_mask != 0
            || node.bounds_type != BoundsType::Default
            || !node.tag_data.is_empty()
        {
            warn!(name: "unhandled_node_attribs", target: "Panda3DLoader",
                "PandaNode attribs attached to node {} are non-zero! Please fix.", node_index);
        }
        if !node.stashed_refs.is_empty() {
            warn!(name: "unexpected_stashed_refs", target: "Panda3DLoader",
                "Node {} has stashed nodes, but this loader doesn't support those. Please fix!", node_index);
        }

        // Finally, let's check if we've already spawned a node to add an AnimationTarget previously. If it
        // isn't in the lookup, then let's spawn a new one.
        let entity = net_nodes
            .and_then(|node_lookup| node_lookup.get(&node_index).copied())
            .unwrap_or_else(|| world.spawn((transform, Name::new(node.name.clone()))).id());

        // Even if the node was already created, it wasn't parented, so parent it now.
        if let Some(parent) = parent {
            world.entity_mut(parent).add_child(entity);
        }

        (entity, effects)
    }

    /// Recursively converts a CharacterJointBundle into the data needed for animating [`SkinnedMesh`]es, as
    /// well as any associated net_nodes.
    async fn convert_joint_bundle(
        &self, world: &mut World, parent: Entity, animation_context: Option<AnimationContext>,
        net_nodes: &mut BTreeMap<usize, Entity>, node_index: usize,
    ) -> (Vec<Mat4>, Vec<Entity>) {
        let mut inverse_bindposes = Vec::new();
        let mut joints = Vec::new();

        match self.nodes.get(node_index) {
            Some(NodeRef::PartBundle(node)) => {
                // We're at a PartBundle/CharacterJointBundle, which means we're attached to a Character and
                // have a skeleton below us. We need to combine this with the PartGroup under us, so we'll
                // grab that and spawn them as one Entity.
                if node.anim_preload_ref.is_some()
                    || node.blend_type != BlendType::NormalizedLinear
                    || node.anim_blend_flag
                    || node.frame_blend_flag
                {
                    warn!(name: "unhandled_part_bundle", target: "Panda3DLoader",
                        "PartBundle attribs on node {} are unhandled, please fix!", node_index);
                }
                // TODO: if we find an instance where this isn't the case, we'll need to spawn a node
                // separately to store each PartGroup, but for now this isn't an issue.
                if node.child_refs.len() != 1 {
                    warn!(name: "unexpected_part_bundle", target: "Panda3DLoader",
                        "Unexpected number of child nodes on PartBundle node {}, ignoring.", node_index);
                }

                let Some(part_group) = self.nodes.get_as::<PartGroup>(node.child_refs[0] as usize) else {
                    warn!(name: "not_a_part_group", target: "Panda3DLoader",
                        "Tried to get node {}, but it wasn't a PartGroup. Unable to create joints, returning.", node.child_refs[0]);
                    return (inverse_bindposes, joints);
                };

                // Create a new node that will serve as the base of any animations we want to play
                if part_group.name != "<skeleton>" {
                    warn!(name: "unexpected_part_group_name", target: "Panda3DLoader",
                        "Encountered a PartGroup that wasn't named <skeleton>, node {}. This model may not be imported correctly.", node.child_refs[0]);
                }
                let name = Name::new(part_group.name.clone());
                let skeleton = world
                    .spawn((
                        AnimationPlayer::default(),
                        Transform::from_matrix(node.root_transform),
                        name.clone(),
                    ))
                    .id();

                // Make sure to parent it correctly
                world.entity_mut(parent).add_child(skeleton);

                inverse_bindposes.push(node.root_transform.inverse());
                joints.push(skeleton);

                // We know this is the root, so create a new `AnimationContext` to keep track of
                // everything, and create a new AnimationTarget
                let animation_context = AnimationContext { root: skeleton, path: smallvec![name] };
                world.entity_mut(skeleton).insert(AnimationTarget {
                    id: AnimationTargetId::from_names(animation_context.path.iter()),
                    player: animation_context.root,
                });

                for child_ref in &part_group.child_refs {
                    Box::pin(self.convert_joint_bundle(
                        world,
                        skeleton,
                        Some(animation_context.clone()),
                        net_nodes,
                        *child_ref as usize,
                    ))
                    .await;
                }
            }
            Some(NodeRef::CharacterJoint(node)) => {
                // We're at an actual skeletal joint.
                let name = Name::new(node.name.clone());
                let joint = world.spawn((Transform::from_matrix(node.default_value), name.clone())).id();

                // Make sure to parent it correctly
                world.entity_mut(parent).add_child(joint);

                inverse_bindposes.push(node.initial_net_transform_inverse);
                joints.push(joint);

                // We should always have a valid AnimationContext, and if we don't, we have bigger worries.
                let mut animation_context = animation_context.unwrap();
                animation_context.path.push(name);
                world.entity_mut(joint).insert(AnimationTarget {
                    id: AnimationTargetId::from_names(animation_context.path.iter()),
                    player: animation_context.root,
                });

                // Check any net transform nodes and try to create them. Theoretically, only ModelNode has the
                // parameter needed to support a transform: Net, so let's just get the node as that.
                for net_node_ref in &node.net_node_refs {
                    let Some(node) = self.nodes.get_as::<ModelNode>(*net_node_ref as usize) else {
                        warn!(name: "not_a_model_node", target: "Panda3DLoader",
                            "Tried to get node {} when trying to construct Net Transforms, but it wasn't a ModelNode, ignoring.", *net_node_ref);
                        continue;
                    };
                    // Spawn a node, add an AnimationTarget to it, so we're able to animate it even if it
                    // doesn't have a mesh. We'll handle its effects and etc once we encounter it normally
                    // in the tree.
                    let name = Name::new(node.name.clone());
                    let transform = self.handle_transform_state(node.transform_ref as usize).await;
                    // Make sure we don't pollute our parent's context
                    let mut animation_context = animation_context.clone();
                    animation_context.path.push(name.clone());
                    let net_node = world
                        .spawn((
                            transform,
                            name,
                            AnimationTarget {
                                id: AnimationTargetId::from_names(animation_context.path.iter()),
                                player: animation_context.root,
                            },
                        ))
                        .id();

                    net_nodes.insert(*net_node_ref as usize, net_node);
                }

                for child_ref in &node.child_refs {
                    Box::pin(self.convert_joint_bundle(
                        world,
                        joint,
                        Some(animation_context.clone()),
                        net_nodes,
                        *child_ref as usize,
                    ))
                    .await;
                }
            }
            Some(node) => println!("Unexpected node {:?} in convert_joint_bundle", node),
            None => {
                warn!(name: "unexpected_node_index", target: "Panda3DLoader",
                    "Tried to access node {}, but it doesn't exist, ignoring.", node_index);
            }
        }

        (inverse_bindposes, joints)
    }

    async fn convert_geom_node(
        &self, loader: &mut AssetLoaderData<'_, '_>, geom_ref: usize, render_ref: usize, parent: Entity,
    ) {
        let Some(geom_node) = self.nodes.get_as::<Geom>(geom_ref) else {
            warn!(name: "invalid_geom_node", target: "Panda3DLoader",
                "Tried to load node {}, but it wasn't a Geom, returning.", geom_ref);
            return;
        };
        let Some(render_state) = self.nodes.get_as::<RenderState>(render_ref) else {
            warn!(name: "invalid_geom_node", target: "Panda3DLoader",
                "Tried to load node {}, but it wasn't a RenderState, returning.", render_ref);
            return;
        };

        // We already handle primitive_type by what type the node_ref is, and we theoretically account for
        // Smooth shading because the mesh already has flat normals calculated. TODO: verify this?
        if geom_node.primitive_refs.len() != 1 {
            warn!(name: "too_many_primitives", target: "Panda3DLoader",
                "More than one primitive is attached to node {}, please fix!", geom_ref);
        }
        if geom_node.bounds_type != BoundsType::Default {
            warn!(name: "bounds_type_unhandled", target: "Panda3DLoader",
                "Geom node {} has a unique BoundsType that isn't being handled, ignoring.", geom_ref);
        }

        let entity = loader.world.spawn(()).id();
        loader.world.entity_mut(parent).add_child(entity);

        // TODO: fold this into here?
        self.convert_primitive(
            loader,
            geom_node.data_ref as usize,
            geom_node.primitive_refs[0] as usize,
            render_state,
            entity,
        )
        .await;
    }

    async fn convert_primitive(
        &self, loader: &mut AssetLoaderData<'_, '_>, data_ref: usize, primitive_ref: usize,
        render_state: &RenderState, entity: Entity,
    ) {
        // First, let's grab the GeomVertexData.
        let Some(vertex_data) = self.nodes.get_as::<GeomVertexData>(data_ref) else {
            warn!(name: "not_a_vertex_data", target: "Panda3DLoader",
                "Tried to load node {}, but it wasn't GeomVertexData, unable to create geometry.", data_ref);
            return;
        };

        // Then, grab the GeomVertexFormat.
        let Some(vertex_format) = self.nodes.get_as::<GeomVertexFormat>(vertex_data.format_ref as usize)
        else {
            warn!(name: "not_a_vertex_format", target: "Panda3DLoader",
                "Tried to load node {}, but it wasn't GeomVertexFormat, unable to create geometry.", vertex_data.format_ref);
            return;
        };

        // Finally, let's grab the GeomPrimitive.
        let Some(primitive) = self.nodes.get_as::<GeomPrimitive>(primitive_ref) else {
            warn!(name: "not_a_geom_primitive", target: "Panda3DLoader",
                "Tried to load node {}, but it wasn't GeomPrimitive, unable to create geometry.", primitive_ref);
            return;
        };

        // Now, let's create a Material.
        if render_state.attrib_refs.is_empty() {
            warn!(name: "no_render_state_attribs", target: "Panda3DLoader",
                "Tried to create a mesh using primitive {}, but it has no attributes and can't be rendered, returning.", primitive_ref);
            return;
        }
        let label = format!("Material{}", loader.assets.materials.len());
        let material = self.create_material(loader, render_state).await;
        let material = loader.context.labeled_asset_scope(label, |_| material);
        loader.assets.materials.push(material.clone());

        loader.world.entity_mut(entity).insert(MeshMaterial3d(material));
    }

    fn convert_wrap_mode(&self, mode: WrapMode, node_index: usize) -> ImageAddressMode {
        match mode {
            WrapMode::Clamp => ImageAddressMode::ClampToEdge,
            WrapMode::Repeat => ImageAddressMode::Repeat,
            WrapMode::Mirror => ImageAddressMode::MirrorRepeat,
            WrapMode::BorderColor => ImageAddressMode::ClampToBorder,
            _ => {
                warn!(name: "unexpected_wrap_mode", target: "Panda3DLoader",
                    "Unsupported WrapMode encountered on node {}", node_index);
                ImageAddressMode::default()
            }
        }
    }

    fn convert_image_filter(&self, filter: FilterType, is_mipmap: bool) -> ImageFilterMode {
        match filter {
            // Direct mappings for basic filtering modes
            FilterType::Nearest => ImageFilterMode::Nearest,
            FilterType::Linear => ImageFilterMode::Linear,

            // These are minification/mipmap modes but might be used for mag filter
            // Map them to their nearest equivalent
            FilterType::NearestMipmapNearest => ImageFilterMode::Nearest,
            FilterType::NearestMipmapLinear => match is_mipmap {
                false => ImageFilterMode::Nearest,
                true => ImageFilterMode::Linear,
            },
            FilterType::LinearMipmapNearest => match is_mipmap {
                false => ImageFilterMode::Linear,
                true => ImageFilterMode::Nearest,
            },
            FilterType::LinearMipmapLinear => ImageFilterMode::Linear,

            // Special cases
            FilterType::Shadow => ImageFilterMode::Linear, // Typically want smooth shadows
            FilterType::Default => ImageFilterMode::Linear, // Most common default
            FilterType::Invalid => ImageFilterMode::Linear, // Safe fallback
        }
    }

    async fn create_material(
        &self, loader: &mut AssetLoaderData<'_, '_>, render_state: &RenderState,
    ) -> Panda3DMaterial {
        let mut material = Panda3DMaterial::default();

        for attrib_ref in &render_state.attrib_refs {
            if attrib_ref.1 != 0 {
                warn!(name: "nonzero_override", target: "Panda3DLoader",
                    "Node {} has a non-zero override value, please fix!", attrib_ref.0);
            }
            match self.nodes.get(attrib_ref.0 as usize) {
                Some(NodeRef::TextureAttrib(attrib)) => {
                    // First, let's validate that we handle all TextureAttrib's fields
                    if attrib.off_all_stages
                        || !attrib.off_stage_refs.is_empty()
                        || attrib.on_stages.len() != 1
                    {
                        warn!(name: "unexpected_texture_attrib", target: "Panda3DLoader",
                            "Creating a Texture using node {}, but it has unexpected on/off nodes, ignoring.", attrib_ref.0);
                        if attrib.on_stages.is_empty() {
                            continue;
                        }
                    }

                    // Let's grab the StageNode inside (hopefully only one!)
                    let stage_node = &attrib.on_stages[0];
                    if stage_node.sampler.is_some()
                        || stage_node.priority != 0
                        || stage_node.implicit_sort != 1
                    {
                        warn!(name: "unexpected_stage_node", target: "Panda3DLoader",
                            "Encountered unexpected StageNode data on node {}, ignoring.", attrib_ref.0);
                    }

                    // Validate that the TextureStage is plain and we can ignore it.
                    let Some(texture_stage) =
                        self.nodes.get_as::<TextureStage>(stage_node.texture_stage_ref as usize)
                    else {
                        warn!(name: "not_a_texture_stage", target: "Panda3DLoader",
                            "Tried to get node {}, but it wasn't a TextureStage, ignoring.", stage_node.texture_stage_ref);
                        continue;
                    };
                    if *texture_stage != TextureStage::default() {
                        warn!(name: "unhandled_texture_stage", target: "Panda3DLoader",
                            "TextureStage Node {} is not the default, please fix!", stage_node.texture_stage_ref);
                    }

                    // Now to grab the Texture and actually handle it
                    let texture_ref = stage_node.texture_ref as usize;
                    // If we've already processed this texture, just load the original Image
                    let image = if let Some(image_id) = loader.image_cache.get(&texture_ref) {
                        loader.assets.textures[*image_id].clone()
                    } else {
                        let Some(texture) = self.nodes.get_as::<Texture>(texture_ref) else {
                            warn!(name: "not_a_texture", target: "Panda3DLoader",
                                "Tried to get node {}, but it wasn't a Texture, ignoring.", texture_ref);
                            continue;
                        };

                        /* I cannot tell if this section is blessed or cursed, fragile or robust, but it
                         * works and that's all I care about */
                        // First, load the RGB image which should always be available
                        let rgb_image = match loader
                            .context
                            .loader()
                            .immediate()
                            .load::<Image>(texture.filename.clone())
                            .await
                        {
                            Ok(image) => image.take(),
                            Err(error) => {
                                warn!(name: "image_file_error", target: "Panda3DLoader",
                                    "Tried to load file {}, got back error {}", texture.filename, error);
                                continue;
                            }
                        };

                        // Then, if the alpha image exists, load it
                        let alpha_image = if !texture.alpha_filename.is_empty() {
                            Some(
                                match loader
                                    .context
                                    .loader()
                                    .immediate()
                                    .load::<Image>(texture.alpha_filename.clone())
                                    .await
                                {
                                    Ok(image) => image.take(),
                                    Err(error) => {
                                        warn!(name: "image_file_error", target: "Panda3DLoader",
                                            "Tried to load file {}, got back error {}", texture.alpha_filename, error);
                                        continue;
                                    }
                                },
                            )
                        } else {
                            None
                        };

                        // If an alpha texture exists, then we need to merge the two into a single Image
                        let mut image = if let Some(alpha_image) = alpha_image {
                            // Image.convert has very limited support, so use a match to filter out the couple
                            // we care about, and convert to RGBA
                            let mut rgb_image = match rgb_image.texture_descriptor.format {
                                TextureFormat::R8Unorm | TextureFormat::Rg8Unorm => {
                                    rgb_image.convert(TextureFormat::Rgba8UnormSrgb).unwrap()
                                }
                                TextureFormat::Rgba8UnormSrgb => rgb_image.clone(),
                                _ => {
                                    warn!(name: "combine_alpha_no_convert", target: "Panda3DLoader",
                                        "Material {} has a separate alpha channel, but the RGB file {} was not in a supported format! Ignoring.", texture_ref, texture.filename);
                                    continue;
                                }
                            };

                            // The only supported format right now is R8, theoretically we could support any
                            // kind of Rgba8 and just grab the alpha from that, TODO?
                            match alpha_image.texture_descriptor.format {
                                TextureFormat::R8Unorm => (),
                                _ => {
                                    warn!(name: "unsupported_alpha_image", target: "Panda3DLoader",
                                        "Trying to merge alpha texture {}, but it's not in a supported format! Ignoring.", texture.alpha_filename);
                                    continue;
                                }
                            }

                            // For the entire image, replace the alpha u8 with the one from alpha image
                            let height = rgb_image.texture_descriptor.size.height;
                            let width = rgb_image.texture_descriptor.size.width;
                            for y in 0..height {
                                for x in 0..width {
                                    let alpha_pixel = alpha_image.data[(y * width + x) as usize];
                                    rgb_image.data[((y * width + x) * 4) as usize + 3] = alpha_pixel;
                                }
                            }
                            rgb_image
                        } else {
                            rgb_image
                        };

                        // Now that we have this new image, we need to configure its properties
                        let descriptor = image.sampler.get_or_init_descriptor();
                        descriptor.label = Some(texture.name.clone());

                        descriptor.address_mode_u = self.convert_wrap_mode(texture.wrap_u, texture_ref);
                        descriptor.address_mode_v = self.convert_wrap_mode(texture.wrap_v, texture_ref);
                        descriptor.address_mode_w = self.convert_wrap_mode(texture.wrap_w, texture_ref);

                        descriptor.mag_filter = self.convert_image_filter(texture.mag_filter, false);
                        descriptor.min_filter = self.convert_image_filter(texture.min_filter, false);
                        descriptor.mipmap_filter = self.convert_image_filter(texture.min_filter, true);

                        // Clamp (-1000..=1000) to (0..=32) since that seems to be the default range for both.
                        // TODO: re-evaluate once we find a model that doesn't have the default?
                        descriptor.lod_min_clamp = (texture.min_lod * 32.0) / 2000.0 + 16.0;
                        descriptor.lod_max_clamp = (texture.max_lod * 32.0) / 2000.0 + 16.0;

                        descriptor.border_color = match texture.border_color.to_array() {
                            [0.0, 0.0, 0.0, 0.0] => Some(ImageSamplerBorderColor::TransparentBlack),
                            [0.0, 0.0, 0.0, 1.0] => Some(ImageSamplerBorderColor::OpaqueBlack),
                            [1.0, 1.0, 1.0, 1.0] => Some(ImageSamplerBorderColor::OpaqueWhite),
                            _ => None,
                        };

                        // Make sure we cache this image so we don't try to merge it again
                        loader.image_cache.insert(texture_ref, loader.assets.textures.len());

                        // Register our (potentially) new image with the AssetServer properly, and store it
                        let label = format!("Image{}", loader.assets.textures.len());
                        let image = loader.context.labeled_asset_scope(label, |_| image);
                        loader.assets.textures.push(image.clone());

                        image
                    };

                    material.base.base_color_texture = Some(image);
                }
                Some(NodeRef::TransparencyAttrib(attrib)) => {
                    material.base.alpha_mode = match attrib.mode {
                        TransparencyMode::None => AlphaMode::Opaque,
                        TransparencyMode::Alpha => AlphaMode::Blend,
                        TransparencyMode::PremultipliedAlpha => AlphaMode::Premultiplied,
                        TransparencyMode::Binary => AlphaMode::Mask(0.5),
                        TransparencyMode::Dual => AlphaMode::AlphaToCoverage,
                        _ => {
                            warn!(name: "multisample_transparency", target: "Panda3DLoader",
                                "Encountered Multisample TransparencyAttrib on node {}, ignoring.", attrib_ref.0);
                            AlphaMode::Opaque
                        }
                    }
                }
                Some(NodeRef::ColorAttrib(attrib)) => {
                    material.base.base_color = match attrib.color_type {
                        // Flat means use this color for all geometry
                        ColorType::Flat => Color::Srgba(Srgba::from_vec4(attrib.color)),
                        // Vertex colors are provided as part of the mesh, just keep base color white
                        ColorType::Vertex => Color::WHITE,
                        // Colors off just means pure white base color.
                        ColorType::Off => Color::WHITE,
                    };
                }
                Some(NodeRef::CullFaceAttrib(attrib)) => {
                    material.base.cull_mode = match attrib.get_effective_mode() {
                        CullMode::None => None,
                        CullMode::Clockwise => Some(Face::Back),
                        CullMode::CounterClockwise => Some(Face::Front),
                        CullMode::Unchanged => unreachable!("get_effective_mode() resolves Unchanged for us"),
                    };
                }
                Some(NodeRef::DepthWriteAttrib(attrib)) => {
                    material.extension.depth_write_enabled = attrib.depth_write_enabled();
                }
                Some(NodeRef::CullBinAttrib(_)) => {
                    // TODO: actually handle this? There's not much we can do about pipelining in this loader.
                }
                Some(node) => println!("Unexpected node {:?} in create_material", node),
                None => {
                    warn!(name: "unexpected_node_index", target: "Panda3DLoader",
                        "Tried to access node {}, but it doesn't exist, ignoring.", attrib_ref.0);
                }
            }
        }

        material
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoadSettings {}

#[derive(Debug, Default)]
pub struct Panda3DLoader;

#[derive(Asset, TypePath, Debug, Default)]
pub struct Panda3DAsset {
    pub scene: Handle<Scene>,
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<Panda3DMaterial>>,
    pub textures: Vec<Handle<Image>>,
    pub bindposes: Vec<Handle<SkinnedMeshInverseBindposes>>,
}

struct AssetLoaderData<'loader, 'context> {
    world: &'loader mut World,
    context: &'loader mut LoadContext<'context>,
    assets: &'loader mut Panda3DAsset,
    // Stores all Texture NodeIDs and their Image# so we don't try to load image files twice
    image_cache: HashMap<usize, usize>,
}

impl AssetLoader for Panda3DLoader {
    type Asset = Panda3DAsset;
    type Error = bam::Error;
    type Settings = LoadSettings;

    async fn load(
        &self, reader: &mut dyn Reader, _settings: &Self::Settings, load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // let start_time = bevy_internal::utils::Instant::now();

        // First, let's parse the data into something we can work with. TODO: take the Reader directly?
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Then, let's parse out our scene graph. TODO: make an async function?
        let bam = BinaryAsset::load(bytes)?;

        // Now we need to post-process it into a scene we can actually spawn
        let mut assets = Self::Asset::default();
        assets.scene = load_context.labeled_asset_scope("Scene0".to_string(), |context| {
            let mut world = World::default();

            let mut loader = AssetLoaderData {
                world: &mut world,
                context,
                assets: &mut assets,
                image_cache: HashMap::new(),
            };

            block_on(bam.recurse_nodes(&mut loader, None, None, None, None, 0));

            Scene::new(world)
        });

        Ok(assets)
    }

    fn extensions(&self) -> &[&str] {
        &["bam", "boo"]
    }
}

pub struct Panda3DPlugin;

impl Plugin for Panda3DPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<Panda3DLoader>()
            .init_asset_loader::<SgiImageLoader>()
            .init_asset::<Panda3DAsset>()
            .add_plugins(MaterialPlugin::<Panda3DMaterial>::default());
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(Panda3DExtensionKey)]
pub struct Panda3DExtension {
    depth_write_enabled: bool,
    decal_effect: bool,
}

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct Panda3DExtensionKey {
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
        _pipeline: &MaterialExtensionPipeline, descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef, key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = key.bind_group_data.depth_write_enabled;
            if key.bind_group_data.decal_effect {
                //TODO: tweak these more if they give any trouble
                depth_stencil.bias.constant = 1;
                depth_stencil.bias.slope_scale = 0.5;
                depth_stencil.depth_write_enabled = false;
            }
        }
        Ok(())
    }
}

impl From<&Panda3DExtension> for Panda3DExtensionKey {
    fn from(extension: &Panda3DExtension) -> Self {
        Self {
            depth_write_enabled: extension.depth_write_enabled,
            decal_effect: extension.decal_effect,
        }
    }
}

pub type Panda3DMaterial = ExtendedMaterial<StandardMaterial, Panda3DExtension>;
