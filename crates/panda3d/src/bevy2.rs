//! This file is designed to provide loading Panda3D BAM assets into the Bevy game engine. This obviously
//! comes with some complexities, as Panda3D and specifically Toontown's scene graph minutia are poorly
//! documented.
//!
//! For example, all Toontown models begin with a ModelRoot or ModelNode that serves as the root node of the
//! .egg file they were converted from. Additionally, specific nodes serve specific purposes. A Character node
//! is designed to be a high level animatable node that multiple meshes attach to, as well as a singular
//! (TODO: check) PartBundle that holds all skinning data

use std::collections::BTreeMap;

use bevy_animation::{
    AnimationClip, AnimationPlayer, AnimationTarget, AnimationTargetId, animated_field,
    animation_curves::{AnimatableCurve, AnimatedField},
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp as _, AssetLoader, Handle, LoadContext, RenderAssetUsages, io::Reader};
use bevy_color::{Color, ColorToComponents as _, Srgba};
use bevy_ecs::{entity::Entity, name::Name, world::World};
use bevy_image::{Image, ImageAddressMode, ImageFilterMode, ImageSamplerBorderColor};
use bevy_math::{EulerRot, curve::UnevenSampleAutoCurve};
use bevy_pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, MaterialPlugin,
    MeshMaterial3d, StandardMaterial,
};
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{
    alpha::AlphaMode,
    mesh::{
        Indices, Mesh, Mesh3d, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues,
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureFormat,
    },
    view::Visibility,
};
use bevy_scene::Scene;
use bevy_tasks::block_on;
use bevy_transform::components::Transform;
use hashbrown::HashMap;
use orthrus_core::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use snafu::prelude::*;
use tracing::warn;

use crate::{
    bevy_sgi::SgiImageLoader,
    nodes::{
        color_attrib::ColorType,
        cull_face_attrib::CullMode,
        dispatch::NodeRef,
        model_node::PreserveTransform,
        part_bundle::BlendType,
        prelude::*,
        sampler_state::{FilterType, WrapMode},
        transform_blend::TransformEntry,
        transform_state::TransformFlags,
        transparency_attrib::TransparencyMode,
    },
    prelude::*,
};

// TODO on this whole file, try to reduce nesting, should be able to create an internal Error type, return
// result and error if we encounter unexpected data, instead of the current stupid if let Some() spam.

#[derive(Debug, Snafu)]
pub enum Panda3DError {
    /// Thrown if an error occurs when decoding a BAM file.
    #[snafu(transparent)]
    Bam { source: bam::Error },

    /// Thrown if an error occurs when trying to read or write files.
    #[snafu(transparent)]
    FileError { source: std::io::Error },

    /// Thrown if unable to directly load an asset.
    #[snafu(transparent)]
    LoadError { source: bevy_asset::LoadDirectError },

    /// Thrown if a [`DataError`] other than EndOfFile is encountered.
    #[snafu(display("Decoding Error {source}"))]
    DataError { source: DataError },

    #[snafu(display("Tried to get node {node_index}, but it wasn't {node_type}!"))]
    WrongNode { node_index: usize, node_type: &'static str },

    #[snafu(display("Tried to parse node {node_index}, but encountered unexpected data!"))]
    UnexpectedData { node_index: usize },
}

impl From<DataError> for Panda3DError {
    #[inline]
    fn from(source: DataError) -> Self {
        Panda3DError::DataError { source }
    }
}

macro_rules! get_node {
    ($self:expr, $node_type:ty, $node_index:expr) => {
        $self
            .nodes
            .get_as::<$node_type>($node_index as usize)
            .context(WrongNodeSnafu { node_index: $node_index as usize, node_type: stringify!($node_type) })
    };
}

#[derive(Debug, Default, Clone, Copy)]
struct Effects {
    is_billboard: bool,
    is_decal: bool,
}

impl Effects {
    async fn new(
        assets: &BinaryAsset, parent: Option<&Effects>, node_index: usize,
    ) -> Result<Self, Panda3DError> {
        let mut result = match parent {
            Some(effects) => *effects,
            None => Self::default(),
        };

        let effects = get_node!(assets, RenderEffects, node_index)?;

        for effect in &effects.effect_refs {
            match assets.nodes.get(*effect as usize) {
                Some(node) => {
                    match node {
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
                    }
                }
                None => {
                    warn!(name: "unexpected_node_index", target: "Panda3DLoader",
                        "Tried to access node {}, but it doesn't exist, ignoring.", effect)
                }
            }
        }

        Ok(result)
    }
}

// Just steal this from bevy_gltf, it's a good structure
#[derive(Clone, Debug)]
struct AnimationContext {
    // The nearest ancestor animation root.
    root: Entity,
    // The path to the animation root. This is used for constructing the animation target UUIDs.
    path: SmallVec<[Name; 8]>,
}

impl BinaryAsset {
    async fn recurse_nodes(
        &self, loader: &mut AssetLoaderData<'_, '_>, parent: Option<Entity>, effects: Option<&Effects>,
        joint_data: Option<&SkinnedMesh>, net_nodes: Option<&BTreeMap<usize, Entity>>, node_index: usize,
    ) -> Result<(), Panda3DError> {
        match self.nodes.get(node_index) {
            Some(NodeRef::ModelNode(node)) => {
                // This can either be a ModelNode or a ModelRoot, either way we need to spawn a new node to
                // attach stuff to.
                let (entity, effects) = self
                    .handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index)
                    .await?;

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
                    .await?;
                }
            }
            Some(NodeRef::PandaNode(node)) => {
                // This is just a plain ol' node, so just process its data and explore all children.
                let (entity, effects) = self
                    .handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index)
                    .await?;

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
                    .await?;
                }
            }
            Some(NodeRef::Character(node)) => {
                // Characters are helper nodes that group together multiple meshes together with animation
                // data. TODO: add a marker Component?
                let (entity, effects) = self
                    .handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index)
                    .await?;

                if node.bundle_refs.len() != 1 {
                    warn!(name: "unexpected_character_node", target: "Panda3DLoader",
                        "Character Node {} has more than one associated CharacterJointBundle, ignoring.", node_index);
                }

                // First, let's process the `CharacterJointBundle` into [`SkinnedMesh`] data, as well as any
                // net nodes we spawned to add an [`AnimationTarget`]. TODO: make a non-recursive function to
                // simplify this mess?
                let mut net_nodes = BTreeMap::new();
                let (inverse_bindposes, joints) = self.convert_joint_bundle(
                    loader,
                    entity,
                    None,
                    &mut net_nodes,
                    node.bundle_refs[0] as usize,
                )?;

                // TODO: migrate to bevy_gltf's new enum-based system so this is less dumb
                let label = format!("Bindpose{}", loader.assets.bindposes.len());
                let inverse_bindposes = loader
                    .context
                    .add_labeled_asset(label, SkinnedMeshInverseBindposes::from(inverse_bindposes));
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
                    .await?;
                }
            }
            Some(NodeRef::AnimBundleNode(node)) => {
                // AnimBundleNodes are helper nodes with an attached AnimBundle that stores an animation. This
                // doesn't technically exist as a node, so let's not create an entity for it.

                if node.draw_control_mask != 0
                    || node.draw_show_mask != 0xFFFFFFFF
                    || node.into_collide_mask != 0
                    || node.bounds_type != BoundsType::Default
                    || !node.tag_data.is_empty()
                    || !node.child_refs.is_empty()
                    || !node.stashed_refs.is_empty()
                {
                    warn!(name: "unhandled_node_attribs", target: "Panda3DLoader",
                        "PandaNode attribs attached to node {} are non-zero! Please fix.", node_index);
                }

                self.convert_anim_bundle(loader, None, None, None, node.anim_bundle_ref as usize)?;
            }
            Some(NodeRef::GeomNode(node)) => {
                // We need to create and attach actual mesh data to this node.
                let (entity, effects) = self
                    .handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index)
                    .await?;

                //TODO handle tags, collide_mask?

                for geom_ref in &node.geom_refs {
                    self.convert_geom_node(
                        loader,
                        joint_data,
                        geom_ref.0 as usize,
                        geom_ref.1 as usize,
                        entity,
                    )
                    .await?;
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
                    .await?;
                }
            }
            Some(NodeRef::SequenceNode(node)) => {
                // SequenceNode handles a special "animation" that swaps visibility on a series of nodes, so
                // we translate that by adding a custom Component, a system that listens for any nodes with
                // that Component, and we set ourselves to Visibility::Hidden so any child nodes inherit that
                // visibility by default, and we can swap whatever the current node is to "play" the
                // animation.
                let (entity, _effects) = self
                    .handle_panda_node(loader.world, parent, effects, net_nodes, node, node_index)
                    .await?;
                if let Some(mut visibility) = loader.world.entity_mut(entity).get_mut::<Visibility>() {
                    *visibility = Visibility::Hidden;
                }
            }
            Some(node) => println!("Unexpected node {:?} in recurse_nodes", node),
            None => {
                warn!(name: "unexpected_node_index", target: "Panda3DLoader",
                    "Tried to access node {}, but it doesn't exist, ignoring.", node_index);
            }
        }
        Ok(())
    }

    /// Constructs a `Transform` from a given `TransformState`. Used for nodes that inherit from `PandaNode`.
    fn handle_transform_state(&self, node_index: usize) -> Transform {
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
    ) -> Result<(Entity, Effects), Panda3DError> {
        // TODO: We don't current handle RenderState, for now, grab it and check if it's empty
        let render_state = get_node!(self, RenderState, node.state_ref as usize)?;

        if !render_state.attrib_refs.is_empty() {
            warn!(name: "unhandled_render_state", target: "Panda3DLoader",
                    "Non-empty RenderState attached to node {} being ignored! Please fix.", node_index);
        }

        // Handle our Transform so we can spawn a new entity
        let transform = self.handle_transform_state(node.transform_ref as usize);

        // We only see what data is attached to a RenderEffects so we can pass it down to child nodes, TODO:
        // figure out proper inheritance
        let effects = Effects::new(self, effects, node.effects_ref as usize).await?;

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
        let entity =
            net_nodes.and_then(|node_lookup| node_lookup.get(&node_index).copied()).unwrap_or_else(|| {
                world.spawn((transform, Visibility::default(), Name::new(node.name.clone()))).id()
            });

        // Even if the node was already created, it wasn't parented, so parent it now.
        if let Some(parent) = parent {
            world.entity_mut(parent).add_child(entity);
        }

        Ok((entity, effects))
    }

    /// Recursively converts a CharacterJointBundle into the data needed for animating [`SkinnedMesh`]es, as
    /// well as any associated net_nodes.
    fn convert_joint_bundle(
        &self, loader: &mut AssetLoaderData<'_, '_>, parent: Entity,
        animation_context: Option<AnimationContext>, net_nodes: &mut BTreeMap<usize, Entity>,
        node_index: usize,
    ) -> Result<(Vec<Mat4>, Vec<Entity>), Panda3DError> {
        let mut inverse_bindposes = Vec::new();
        let mut joints = Vec::new();

        match self.nodes.get(node_index) {
            Some(NodeRef::PartBundle(node)) => {
                // We're at a PartBundle/CharacterJointBundle, which means we're attached to a Character and
                // are the root node of a skeleton. We'll need to attach the AnimationPlayer to our parent
                // (the Character) Entity so our AnimationPath includes it, which may otherwise have
                // unexpected side effects when trying to play animations on nodes that share a name.

                // Let's start by validating the PartBundle, which should share the same name as the Character
                // above us.
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

                // Let's also grab the PartGroup so we can make sure it's what we expect.
                let part_group = get_node!(self, PartGroup, node.child_refs[0] as usize)?;

                if part_group.name != "<skeleton>" {
                    warn!(name: "unexpected_part_group_name", target: "Panda3DLoader",
                        "Encountered a PartGroup that wasn't named <skeleton>, node {}. This model may not be imported correctly.", node.child_refs[0]);
                }

                // We first need to create a new AnimationPlayer and attach it to our parent. It cannot have
                // animation tables assigned to itself, so we'll only add an AnimationTarget to the skeleton
                // on down.
                loader.world.entity_mut(parent).insert(AnimationPlayer::default());
                loader.assets.animators.push(parent);

                let parent_name = Name::new(node.name.clone());
                let name = Name::new(part_group.name.clone());
                let animation_context =
                    AnimationContext { root: parent, path: smallvec![parent_name, name.clone()] };

                let skeleton = loader
                    .world
                    .spawn((
                        AnimationTarget {
                            id: AnimationTargetId::from_names(animation_context.path.iter()),
                            player: animation_context.root,
                        },
                        Transform::from_matrix(node.root_transform),
                        Visibility::default(),
                        name.clone(),
                    ))
                    .id();

                // Make sure to parent it correctly
                loader.world.entity_mut(parent).add_child(skeleton);

                inverse_bindposes.push(node.root_transform.inverse());
                joints.push(skeleton);

                for child_ref in &part_group.child_refs {
                    let (child_inverse_bindposes, child_joints) = self.convert_joint_bundle(
                        loader,
                        skeleton,
                        Some(animation_context.clone()),
                        net_nodes,
                        *child_ref as usize,
                    )?;
                    inverse_bindposes.extend(child_inverse_bindposes);
                    joints.extend(child_joints);
                }
            }
            Some(NodeRef::CharacterJoint(node)) => {
                // We're at an actual skeletal joint.

                let name = Name::new(node.name.clone());
                let joint = loader
                    .world
                    .spawn((Transform::from_matrix(node.default_value), Visibility::default(), name.clone()))
                    .id();

                // Make sure to parent it correctly
                loader.world.entity_mut(parent).add_child(joint);

                inverse_bindposes.push(node.initial_net_transform_inverse);
                joints.push(joint);

                // We should always have a valid AnimationContext, and if we don't, we have bigger worries.
                let mut animation_context = animation_context.unwrap();
                animation_context.path.push(name);
                loader.world.entity_mut(joint).insert(AnimationTarget {
                    id: AnimationTargetId::from_names(animation_context.path.iter()),
                    player: animation_context.root,
                });

                // Check any net transform nodes and try to create them. Theoretically, only ModelNode has the
                // parameter needed to support a transform: Net, so let's just get the node as that.
                for net_node_ref in &node.net_node_refs {
                    let node = get_node!(self, ModelNode, *net_node_ref as usize)?;

                    // Spawn a node, add an AnimationTarget to it, so we're able to animate it even if it
                    // doesn't have a mesh. We'll handle its effects and etc once we encounter it normally in
                    // the tree.
                    let name = Name::new(node.name.clone());
                    let transform = self.handle_transform_state(node.transform_ref as usize);
                    // Make sure we don't pollute our parent's context
                    let mut animation_context = animation_context.clone();
                    animation_context.path.push(name.clone());
                    let net_node = loader
                        .world
                        .spawn((
                            transform,
                            Visibility::default(),
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
                    let (child_inverse_bindposes, child_joints) = self.convert_joint_bundle(
                        loader,
                        joint,
                        Some(animation_context.clone()),
                        net_nodes,
                        *child_ref as usize,
                    )?;
                    inverse_bindposes.extend(child_inverse_bindposes);
                    joints.extend(child_joints);
                }
            }
            Some(node) => println!("Unexpected node {:?} in convert_joint_bundle", node),
            None => {
                warn!(name: "unexpected_node_index", target: "Panda3DLoader",
                    "Tried to access node {}, but it doesn't exist, ignoring.", node_index);
            }
        }

        Ok((inverse_bindposes, joints))
    }

    async fn convert_geom_node(
        &self, loader: &mut AssetLoaderData<'_, '_>, joint_data: Option<&SkinnedMesh>, geom_ref: usize,
        render_ref: usize, parent: Entity,
    ) -> Result<(), Panda3DError> {
        let geom_node = get_node!(self, Geom, geom_ref)?;
        let render_state = get_node!(self, RenderState, render_ref)?;

        let entity = loader.world.spawn((Transform::default(), Visibility::default())).id();
        loader.world.entity_mut(parent).add_child(entity);

        // Now, let's create a Material.
        let label = format!("Material{}", loader.assets.materials.len());
        // This should be fine, if attrib_refs is empty, it'll just return a default Material.
        let material = self.create_material(loader, render_state).await?;
        let material = loader.context.add_labeled_asset(label, material);
        loader.assets.materials.push(material.clone());

        // TODO: remove unwrap
        let label = format!("Mesh{}", loader.assets.meshes.len());
        let mesh = self.create_mesh(loader, joint_data, entity, geom_ref, geom_node).unwrap();
        let mesh = loader.context.add_labeled_asset(label, mesh);
        loader.assets.meshes.push(mesh.clone());

        loader.world.entity_mut(entity).insert((Mesh3d(mesh), MeshMaterial3d(material)));

        Ok(())
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
            FilterType::NearestMipmapLinear => {
                match is_mipmap {
                    false => ImageFilterMode::Nearest,
                    true => ImageFilterMode::Linear,
                }
            }
            FilterType::LinearMipmapNearest => {
                match is_mipmap {
                    false => ImageFilterMode::Linear,
                    true => ImageFilterMode::Nearest,
                }
            }
            FilterType::LinearMipmapLinear => ImageFilterMode::Linear,

            // Special cases
            FilterType::Shadow => ImageFilterMode::Linear, // Typically want smooth shadows
            FilterType::Default => ImageFilterMode::Linear, // Most common default
            FilterType::Invalid => ImageFilterMode::Linear, // Safe fallback
        }
    }

    async fn create_material(
        &self, loader: &mut AssetLoaderData<'_, '_>, render_state: &RenderState,
    ) -> Result<Panda3DMaterial, Panda3DError> {
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
                    let texture_stage = get_node!(self, TextureStage, stage_node.texture_stage_ref as usize)?;
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
                        let texture = get_node!(self, Texture, texture_ref)?;

                        /* I cannot tell if this section is blessed or cursed, fragile or robust,
                         * but it works and that's all I care about */
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

                        // If an alpha texture exists, then we need to merge the two into a single Image.
                        // TODO: enforce texture.format?
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
                            if let (Some(alpha_data), Some(rgb_data)) =
                                (alpha_image.data.as_ref(), rgb_image.data.as_mut())
                            {
                                for y in 0..height {
                                    for x in 0..width {
                                        let alpha_pixel = alpha_data[(y * width + x) as usize];
                                        rgb_data[((y * width + x) * 4) as usize + 3] = alpha_pixel;
                                    }
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
                        let image = loader.context.add_labeled_asset(label, image);
                        loader.assets.textures.push(image.clone());

                        image
                    };

                    // TODO: not always base_color_texture, see egg MODULATE
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
                        CullMode::Unchanged => {
                            unreachable!("get_effective_mode() resolves Unchanged for us")
                        }
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

        //TODO: create toggle when loading so users can choose to use actual lighting
        material.base.unlit = true;
        material.base.perceptual_roughness = 1.0;
        material.base.fog_enabled = false;

        Ok(material)
    }

    fn convert_blend_entry(&self, entry: &TransformEntry, lookup: &HashMap<u32, u16>) -> Option<(u16, f32)> {
        lookup.get(&entry.transform_ref).map(|&joint_id| (joint_id, entry.weight))
    }

    fn process_blend(&self, blend: &TransformBlend, lookup: &HashMap<u32, u16>) -> ([u16; 4], [f32; 4]) {
        let mut indices = [0u16; 4];
        let mut weights = [0f32; 4];

        // First sort entries by weight
        let mut entries: Vec<_> =
            blend.entries.iter().filter_map(|entry| self.convert_blend_entry(entry, lookup)).collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Take first 4 entries after sorting
        for (i, &(joint_id, weight)) in entries.iter().take(4).enumerate() {
            indices[i] = joint_id;
            weights[i] = weight;
        }

        // Normalize weights
        let total: f32 = weights.iter().sum();
        if total > 0.0 {
            weights.iter_mut().for_each(|w| *w /= total);
        }

        (indices, weights)
    }

    fn build_joint_lookup(
        &self, blend_table: &TransformBlendTable, world: &World, joint_data: &SkinnedMesh,
    ) -> Result<HashMap<u32, u16>, Panda3DError> {
        let mut lookup = HashMap::new();

        for transform in &blend_table.blends {
            for entry in &transform.entries {
                if lookup.contains_key(&entry.transform_ref) {
                    continue;
                }

                let vertex_transform = get_node!(self, JointVertexTransform, entry.transform_ref as usize)?;
                let joint = get_node!(self, CharacterJoint, vertex_transform.joint_ref as usize)?;

                // Find matching joint in joint_data
                for (joint_id, &entity) in joint_data.joints.iter().enumerate() {
                    if **world.entity(entity).get::<Name>().unwrap() == *joint.name {
                        lookup.insert(entry.transform_ref, joint_id as u16);
                        break;
                    }
                }
            }
        }

        Ok(lookup)
    }

    fn create_mesh(
        &self, loader: &mut AssetLoaderData<'_, '_>, joint_data: Option<&SkinnedMesh>, entity: Entity,
        geom_ref: usize, geom_node: &Geom,
    ) -> Result<Mesh, Panda3DError> {
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

        // First, let's grab the GeomVertexData.
        let vertex_data = get_node!(self, GeomVertexData, geom_node.data_ref as usize)?;

        // Then, grab the GeomVertexFormat.
        let vertex_format = get_node!(self, GeomVertexFormat, vertex_data.format_ref as usize)?;

        // Finally, let's grab the GeomPrimitive.
        let primitive = get_node!(self, GeomPrimitive, geom_node.primitive_refs[0] as usize)?;

        let topology = if geom_node.geom_rendering.contains(GeomRendering::TriangleStrip) {
            PrimitiveTopology::TriangleStrip
        } else if geom_node.geom_rendering.is_empty() {
            PrimitiveTopology::TriangleList
        } else {
            warn!(name: "unexpected_rendering_flags", target: "Panda3DLoader",
                "Unknown geometry rendering type: {:?}, defaulting to TriangleList", geom_node.geom_rendering);
            PrimitiveTopology::TriangleList
        };

        let mut mesh = Mesh::new(topology, RenderAssetUsages::default());

        match primitive.vertices_ref {
            // If we have an associated ArrayData, then this polygon is indexed, so we need to read it
            Some(index) => {
                let array_data = get_node!(self, GeomVertexArrayData, index as usize)?;
                let array_format =
                    get_node!(self, GeomVertexArrayFormat, array_data.array_format_ref as usize)?;

                if array_format.num_columns != 1 {
                    warn!(name: "too_many_array_columns", target: "Panda3DLoader",
                        "Too many columns in GeomVertexArrayFormat {}, ignoring.", array_data.array_format_ref as usize);
                }

                // Grab the column, and validate that it's what we expect.
                let column = &array_format.columns[0];
                let internal_name = get_node!(self, InternalName, column.name_ref as usize)?;

                ensure!(
                    column.numeric_type == NumericType::U16
                        && column.contents == Contents::Index
                        && internal_name.name == "index",
                    UnexpectedDataSnafu { node_index: column.name_ref as usize },
                );

                let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);
                let mut indices = Vec::with_capacity(data.len().unwrap() as usize / 2);
                for _ in 0..indices.capacity() {
                    indices.push(data.read_u16()?);
                }
                mesh.insert_indices(Indices::U16(indices));
            }
            // Otherwise, we need to generate indices ourselves
            None => {
                // TODO: make sure this is robust?
                let start = primitive.first_vertex as u32;
                let end = match primitive.num_vertices {
                    -1 => {
                        if let Some(ends_ref) = primitive.ends_ref {
                            let ends = &self.arrays[ends_ref as usize];
                            ensure!(
                                ends.len() == 1,
                                UnexpectedDataSnafu { node_index: geom_node.primitive_refs[0] as usize }
                            );
                            ends[0]
                        } else {
                            return Err(Panda3DError::UnexpectedData {
                                node_index: geom_node.primitive_refs[0] as usize,
                            });
                        }
                    }
                    num_vertices => num_vertices as u32,
                };
                mesh.insert_indices(Indices::U32((start..start + end).collect()));
            }
        }

        // Now let's process the sub-arrays. We always have at least one, containing the actual mesh data.
        let array_data = get_node!(self, GeomVertexArrayData, vertex_data.array_refs[0] as usize)?;
        let array_format = get_node!(self, GeomVertexArrayFormat, vertex_format.array_refs[0] as usize)?;

        // Let's manually calculate the number of polygons/primitives, since it's a bit of a mess otherwise.
        let num_primitives = array_data.buffer.len() as u64 / u64::from(array_format.stride);
        let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);
        for column in &array_format.columns {
            let internal_name = get_node!(self, InternalName, column.name_ref as usize)?;
            match internal_name.name.as_str() {
                "vertex" => {
                    // Note: this can be 4D homogenous space; if it is, just ignore the 4th float since it's
                    // 1.0.
                    if (column.num_components != 3 && column.num_components != 4)
                        || column.numeric_type != NumericType::F32
                        || column.contents != Contents::Point
                    {
                        warn!(name: "unexpected_vertex_type", target: "Panda3DLoader",
                            "Tried to parse vertex data on node {}, but encountered unexpected data, ignoring.", vertex_data.array_refs[0]);
                        continue;
                    }

                    let mut vertex_data = Vec::with_capacity(num_primitives as usize);
                    for n in 0..num_primitives {
                        // We have a stride to worry about
                        data.set_position(u64::from(column.start) + u64::from(array_format.stride) * n)?;
                        vertex_data.push([data.read_f32()?, data.read_f32()?, data.read_f32()?]);
                    }
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_data);
                }
                "texcoord" => {
                    if column.num_components != 2
                        || column.numeric_type != NumericType::F32
                        || column.contents != Contents::TexCoord
                    {
                        warn!(name: "unexpected_texcoord_type", target: "Panda3DLoader",
                            "Tried to parse texcoord data on node {}, but encountered unexpected data, ignoring.", vertex_data.array_refs[0]);
                        continue;
                    }

                    let mut texcoord_data = Vec::with_capacity(num_primitives as usize);
                    for n in 0..num_primitives {
                        // We have a stride to worry about
                        data.set_position(u64::from(array_format.stride) * n + u64::from(column.start))?;

                        // Panda3D stores flipped Y values to support OpenGL, so we do 1.0 - value.
                        texcoord_data.push([data.read_f32()?, 1.0 - data.read_f32()?]);
                    }
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, texcoord_data);
                }
                _ => {
                    warn!(name: "unexpected_column_type", target: "Panda3DLoader",
                    "Unexpected Column Type Encountered: {}, ignoring.", internal_name.name)
                }
            }
        }

        // Now that we've handled base data, let's check all other tables.
        let mut tables_read = 1;
        if let Some(_node_index) = vertex_data.transform_table_ref {
            warn!(name: "unsupported_transform_table", target: "Panda3DLoader",
                "Vertex Data {} has a TransformTable, please fix!", geom_node.data_ref);
            tables_read += 1;
        }

        // if vertex_data.has_column("transform_blend") && joint_map.is_some() &&
        // transform_blend_table.is_some() do shit; TODO make a reader for
        // GeomVertexColumn::Packer::get_data1i that uses a match instead of hardcoded bullshit. Follow
        // EggSaver::convert_primitive more closely.
        if let Some(node_index) = vertex_data.transform_blend_table_ref {
            let blend_table = get_node!(self, TransformBlendTable, node_index as usize)?;

            // We first need to build a HashMap lookup that maps this BAM's ObjectID->Joint Index, so we can
            // take a shortcut when filling out ATTRIBUTE_JOINT_WEIGHT and ATTRIBUTE_JOINT_INDEX.
            //
            // We have to walk the TransformBlendTable twice, but the number of joints is less than the number
            // of blend combinations, so this should overall save time.
            let lookup = match joint_data {
                Some(joint_data) => self.build_joint_lookup(blend_table, loader.world, joint_data)?,
                None => return Ok(mesh),
            };

            // Process blend data ahead of time for each unique blend
            let transforms: Vec<([u16; 4], [f32; 4])> =
                blend_table.blends.iter().map(|blend| self.process_blend(blend, &lookup)).collect();

            // Read node's array data to get blend indices
            let node_index = vertex_data.array_refs[tables_read] as usize;
            let array_data = get_node!(self, GeomVertexArrayData, node_index)?;

            let node_index = array_data.array_format_ref as usize;
            let array_format = get_node!(self, GeomVertexArrayFormat, node_index)?;

            let mut data = DataCursorRef::new(&array_data.buffer, Endian::Little);
            let mut blend_lookup = vec![[0u16; 4]; num_primitives as usize];
            let mut blend_table = vec![[0f32; 4]; num_primitives as usize];

            for n in 0..num_primitives {
                data.set_position(u64::from(array_format.stride) * n)?;
                let lookup_id = data.read_u16()? as usize;
                blend_lookup[n as usize] = transforms[lookup_id].0;
                blend_table[n as usize] = transforms[lookup_id].1;
            }

            mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(blend_lookup));
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_JOINT_WEIGHT,
                VertexAttributeValues::Float32x4(blend_table),
            );
            if let Some(joint_data) = joint_data {
                loader.world.entity_mut(entity).insert(joint_data.clone());
            }

            //tables_read += 1;
        }
        Ok(mesh)
    }

    fn convert_anim_bundle(
        &self, loader: &mut AssetLoaderData<'_, '_>, animation: Option<&mut AnimationClip>,
        animation_context: Option<AnimationContext>, frame_data: Option<(usize, f32)>, node_index: usize,
    ) -> Result<(), Panda3DError> {
        fn expand_channel_data(table: &[f32], default: f32, num_frames: usize) -> Vec<f32> {
            match table.len() {
                0 => vec![default; num_frames],
                1 => vec![table[0]; num_frames],
                _ => table.to_vec(),
            }
        }

        match self.nodes.get(node_index) {
            Some(NodeRef::AnimBundle(node)) => {
                // We're at the base of an AnimBundle, so let's start building an AnimationClip
                let mut animation = AnimationClip::default();
                animation.set_duration(f32::from(node.num_frames) / node.fps);

                // Let's also pull up the AnimGroups, since we know what they look like
                if node.child_refs.len() != 2 {
                    warn!(name: "unexpected_anim_bundle", target: "Panda3DLoader",
                        "Unexpected number of child nodes on Node {}, unable to make animation!", node_index);
                    return Ok(());
                }

                // Then, let's process skeleton/transform animation data
                let skeleton = get_node!(self, AnimGroup, node.child_refs[0] as usize)?;
                if skeleton.name != "<skeleton>" {
                    warn!(name: "unexpected_anim_group", target: "Panda3DLoader",
                        "Expected node {} to have <skeleton> as a name but it didn't, ignoring.", node.child_refs[0]);
                }

                let animation_context = AnimationContext {
                    root: Entity::PLACEHOLDER,
                    path: smallvec![Name::new(node.name.clone()), Name::new(skeleton.name.clone())],
                };

                for child_ref in &skeleton.child_refs {
                    self.convert_anim_bundle(
                        loader,
                        Some(&mut animation),
                        Some(animation_context.clone()),
                        Some((node.num_frames as usize, node.fps)),
                        *child_ref as usize,
                    )?;
                }

                // Finally, let's process morph target animations
                let morph = get_node!(self, AnimGroup, node.child_refs[1] as usize)?;
                if !morph.child_refs.is_empty() {
                    warn!(name: "morph_anims_unimplemented", target: "Panda3DLoader",
                        "Node {} has Morph Target Animations, but they're currently unimplemented, please fix!", node_index);
                }

                let label = format!("Animation{}", loader.assets.animations.len());
                let clip = loader.context.add_labeled_asset(label, animation);
                loader.assets.animations.push(clip);
            }
            Some(NodeRef::AnimChannelMatrixXfmTable(node)) => {
                if let (Some(mut animation_context), Some(animation)) = (animation_context, animation) {
                    let name = Name::new(node.name.clone());
                    animation_context.path.push(name);

                    println!("Animation {:?}", animation_context.path);

                    let anim_target_id = AnimationTargetId::from_names(animation_context.path.iter());

                    let (num_frames, fps) = frame_data.unwrap();
                    let frame_times = (0..num_frames).map(|i| i as f32 / fps);

                    // Let's just check shear now since it's easier
                    if !node.tables[3].is_empty() || !node.tables[4].is_empty() || !node.tables[5].is_empty()
                    {
                        warn!(name: "shear_animation_unsupported", target: "Panda3DLoader",
                            "Shear animation detected on node {}, currently unsupported.", node_index);
                    }

                    for n in [0, 2, 3] {
                        let default = match n {
                            0 => 1.0, // Scale
                            2 => 0.0, // Rotation
                            3 => 0.0, // Translation
                            _ => unreachable!(),
                        };

                        let channels = [
                            expand_channel_data(&node.tables[n * 3], default, num_frames),
                            expand_channel_data(&node.tables[n * 3 + 1], default, num_frames),
                            expand_channel_data(&node.tables[n * 3 + 2], default, num_frames),
                        ];

                        if !channels[0].is_empty() || !channels[1].is_empty() || !channels[2].is_empty() {
                            match n {
                                0 => {
                                    // Scale
                                    let scale_values: Vec<Vec3> = (0..num_frames)
                                        .map(|i| Vec3::new(channels[0][i], channels[1][i], channels[2][i]))
                                        .collect();

                                    animation.add_curve_to_target(
                                        anim_target_id,
                                        AnimatableCurve::new(
                                            animated_field!(Transform::scale),
                                            UnevenSampleAutoCurve::new(frame_times.clone().zip(scale_values))
                                                .unwrap(),
                                        ),
                                    );
                                }
                                2 => {
                                    // Rotation
                                    let rotation_values: Vec<Quat> = (0..num_frames)
                                        .map(|i| {
                                            Quat::from_euler(
                                                EulerRot::ZXY,
                                                channels[0][i].to_radians(), // heading
                                                channels[1][i].to_radians(), // pitch
                                                channels[2][i].to_radians(), // roll
                                            )
                                        })
                                        .collect();

                                    animation.add_curve_to_target(
                                        anim_target_id,
                                        AnimatableCurve::new(
                                            animated_field!(Transform::rotation),
                                            UnevenSampleAutoCurve::new(
                                                frame_times.clone().zip(rotation_values),
                                            )
                                            .unwrap(),
                                        ),
                                    );
                                }
                                3 => {
                                    // Translation
                                    let translation_values: Vec<Vec3> = (0..num_frames)
                                        .map(|i| Vec3::new(channels[0][i], channels[1][i], channels[2][i]))
                                        .collect();

                                    animation.add_curve_to_target(
                                        anim_target_id,
                                        AnimatableCurve::new(
                                            animated_field!(Transform::translation),
                                            UnevenSampleAutoCurve::new(
                                                frame_times.clone().zip(translation_values),
                                            )
                                            .unwrap(),
                                        ),
                                    );
                                }
                                _ => unreachable!(),
                            }
                        }
                    }

                    for child_ref in &node.child_refs {
                        self.convert_anim_bundle(
                            loader,
                            Some(animation),
                            Some(animation_context.clone()),
                            frame_data,
                            *child_ref as usize,
                        )?;
                    }
                }
            }
            Some(node) => println!("Unexpected node {:?} in convert_anim_bundle", node),
            None => {
                warn!(name: "unexpected_node_index", target: "Panda3DLoader",
                    "Tried to access node {}, but it doesn't exist, ignoring.", node_index);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoadSettings {}

#[derive(Debug, Default)]
pub struct PandaLoader;

#[derive(Asset, TypePath, Debug, Default)]
pub struct PandaAsset {
    pub scene: Handle<Scene>,
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<Panda3DMaterial>>,
    pub textures: Vec<Handle<Image>>,
    pub bindposes: Vec<Handle<SkinnedMeshInverseBindposes>>,
    /// All entities that have an AnimationPlayer attached
    pub animators: Vec<Entity>,
    pub animations: Vec<Handle<AnimationClip>>,
}

struct AssetLoaderData<'loader, 'context> {
    world: &'loader mut World,
    context: &'loader mut LoadContext<'context>,
    assets: &'loader mut PandaAsset,
    // Stores all Texture NodeIDs and their Image# so we don't try to load image files twice
    image_cache: HashMap<usize, usize>,
}

impl AssetLoader for PandaLoader {
    type Asset = PandaAsset;
    type Error = Panda3DError;
    type Settings = LoadSettings;

    async fn load(
        &self, reader: &mut dyn Reader, _settings: &Self::Settings, load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // let start_time = bevy_internal::utils::Instant::now();

        // First, let's parse the data into something we can work with. TODO: take the Reader directly?
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Then, let's parse out our scene graph.
        let bam = BinaryAsset::load(bytes)?;

        // Now we need to post-process it into a scene the user can actually spawn
        let mut assets = Self::Asset::default();
        let mut world = World::default();

        let mut loader = AssetLoaderData {
            world: &mut world,
            context: load_context,
            assets: &mut assets,
            image_cache: HashMap::new(),
        };

        // Let's first pull out the root node, since it's a placeholder.
        let root_node = get_node!(bam, ModelNode, 0)?;

        if root_node.draw_control_mask != 0
            || root_node.draw_show_mask != 0xFFFFFFFF
            || root_node.into_collide_mask != 0
            || root_node.bounds_type != BoundsType::Default
            || root_node.transform != PreserveTransform::None
            || root_node.attributes != 0
            || root_node.child_refs.len() != 1
        {
            warn!(name: "unexpected_root_node", target: "Panda3DLoader", "Root Node doesn't have default parameters! May not be loaded correctly.");
        }

        block_on(bam.recurse_nodes(&mut loader, None, None, None, None, root_node.child_refs[0].0 as usize))?;

        assets.scene = load_context.add_labeled_asset("Scene0".to_string(), Scene::new(world));

        Ok(assets)
    }

    fn extensions(&self) -> &[&str] {
        &["bam", "boo"]
    }
}

struct PandaImage;

#[derive(Default, Serialize, Deserialize)]
struct PandaImageInfo {
    filename: String,
    alpha_filename: String,
}

impl AssetLoader for PandaImage {
    type Asset = Image;
    type Error = Panda3DError;
    type Settings = PandaImageInfo;

    async fn load(
        &self, reader: &mut dyn Reader, settings: &Self::Settings, load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let rgb_image = load_context.loader().immediate().load::<Image>(&*settings.filename).await?.take();
        let alpha_image = match &*settings.alpha_filename {
            "" => None,
            filename => Some(load_context.loader().immediate().load::<Image>(filename).await?.take()),
        };

        Ok(rgb_image)
    }

    // Loads as panda://relativepath.image, with the raw parameters as settings
    fn extensions(&self) -> &[&str] {
        &["image"]
    }
}

/*
#[derive(Clone)]
enum PandaStorage {
    Extracted { root: std::path::PathBuf },
    Multifile { phases: BTreeMap<String, Multifile> },
}

#[derive(Clone)]
pub struct PandaAssetReader {
    storage: PandaStorage,
}

impl PandaAssetReader {
    fn new_extracted(root: PathBuf) -> Self {
        Self { storage: PandaStorage::Extracted { root } }
    }

    fn new_multifile(phases: BTreeMap<String, Multifile>) -> Self {
        Self { storage: PandaStorage::Multifile { phases } }
    }
}

impl AssetReader for PandaAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader + 'a>, AssetReaderError> {
        println!("AssetReader::read(path: {path:?})");
        match &self.storage {
            PandaStorage::Extracted { root } => {
                let full_path = root.join(path);
                let data = std::fs::read(full_path)?;
                Ok(Box::new(VecReader::new(data)))
            }
            PandaStorage::Multifile { phases } => {
                // Assuming we use paths ala `panda://phase_4/maps/texture.png` and we get the raw path
                let path_str = path.to_str().ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?;
                // First folder will always be a Multifile phase filename
                let (phase, file_path) =
                    path_str.split_once('/').ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?;

                let file = phases
                    .get(phase)
                    .and_then(|mf| mf.files.get(file_path))
                    .ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?;

                Ok(Box::new(SliceReader::new(&file.data)))
            }
        }
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        match &self.storage {
            PandaStorage::Extracted { root } => {
                let mut meta_path = root.join(path);
                let mut extension = path.extension().unwrap_or_default().to_os_string();
                extension.push(".meta");
                meta_path.set_extension(extension);
                let data = std::fs::read(meta_path)?;
                Ok(VecReader::new(data))
            }
            PandaStorage::Multifile { .. } => {
                // Multifiles don't support .meta files
                Err(AssetReaderError::NotFound(path.to_owned()))
            }
        }
    }

    async fn read_directory<'a>(&'a self, path: &'a Path) -> Result<Box<PathStream>, AssetReaderError> {
        match &self.storage {
            PandaStorage::Extracted { root } => {
                let full_path = root.join(path);
                match std::fs::read_dir(full_path) {
                    Ok(read_dir) => {
                        let paths = read_dir.filter_map(|entry| entry.ok()).map(|entry| entry.path());

                        Ok(Box::new(stream::iter(paths)))
                    }
                    Err(error) => Err(error.into()),
                }
            }
            PandaStorage::Multifile { phases } => {
                if path.as_os_str().is_empty() || path == Path::new("/") {
                    let paths = phases.keys().map(|phase| PathBuf::from(phase)).collect::<Vec<_>>();

                    return Ok(Box::new(stream::iter(paths)));
                }

                Err(AssetReaderError::NotFound(path.to_owned()))
            }
        }
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        match &self.storage {
            PandaStorage::Extracted { root } => {
                let full_path = root.join(path);
                match std::fs::metadata(&full_path) {
                    Ok(meta) => Ok(meta.is_dir()),
                    Err(error) => Err(error.into()),
                }
            }
            PandaStorage::Multifile { phases } => {
                if path.as_os_str().is_empty() || path == Path::new("/") {
                    return Ok(true);
                }
                if let Some(phase) = path.to_str() {
                    return Ok(phases.contains_key(phase));
                }
                Ok(false)
            }
        }
    }
}
*/
pub struct Panda3DPlugin;

impl Plugin for Panda3DPlugin {
    fn build(&self, app: &mut App) {
        /*let reader = if cfg!(debug_assertions) {
            PandaAssetReader::new_extracted("assets".into())
        } else {
            PandaAssetReader::new_multifile(BTreeMap::new())
        };

        app.register_asset_source(
            "panda",
            AssetSource::build()
                .with_reader(move || Box::new(reader.clone()))
                .with_watch_warning("Some assets cannot be hot-reloaded"),
        )*/
        app.init_asset_loader::<PandaLoader>()
            .init_asset_loader::<SgiImageLoader>()
            .init_asset::<PandaAsset>()
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
        Self { depth_write_enabled: extension.depth_write_enabled, decal_effect: extension.decal_effect }
    }
}

pub type Panda3DMaterial = ExtendedMaterial<StandardMaterial, Panda3DExtension>;
