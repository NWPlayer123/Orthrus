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
use bevy_internal::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy_internal::prelude::*;
use bevy_internal::render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_internal::render::mesh::MeshVertexBufferLayoutRef;
use bevy_internal::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy_internal::tasks::block_on;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use crate::nodes::dispatch::NodeRef;
use crate::nodes::part_bundle::BlendType;
use crate::nodes::prelude::*;
use crate::nodes::transform_state::TransformFlags;
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

        if let Some(effects) = assets.nodes.get_as::<RenderEffects>(node_index) {
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
        } else {
            warn!(name: "not_a_render_effects", target: "Panda3DLoader",
                "Tried to access node {}, but it's not a RenderEffects, ignoring.", node_index);
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
        &self, loader_data: &mut AssetLoaderData<'_, '_>, parent: Option<Entity>, effects: Option<&Effects>,
        joint_data: Option<&SkinnedMesh>, net_nodes: Option<&BTreeMap<usize, Entity>>, node_index: usize,
    ) {
        match self.nodes.get(node_index) {
            Some(NodeRef::ModelNode(node)) => {
                // This can either be a ModelNode or a ModelRoot, either way we need to spawn a new node to
                // attach stuff to.
                let (entity, effects) = self
                    .handle_panda_node(loader_data.world, parent, effects, net_nodes, node, node_index)
                    .await;

                for child_ref in &node.child_refs {
                    if child_ref.1 != 0 {
                        warn!(name: "nonzero_node_sort", target: "Panda3DLoader",
                                    "Node {} has a child with non-zero sort order, please fix!", node_index);
                    }
                    Box::pin(self.recurse_nodes(
                        loader_data,
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
                let (entity, effects) = self
                    .handle_panda_node(loader_data.world, parent, effects, net_nodes, node, node_index)
                    .await;

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
                        loader_data.world,
                        entity,
                        None,
                        &mut net_nodes,
                        node.bundle_refs[0] as usize,
                    )
                    .await;

                // TODO: migrate to bevy_gltf's new enum-based system so this is less dumb
                let label = format!("Bindpose{}", loader_data.assets.bindposes.len());
                let inverse_bindposes = loader_data
                    .context
                    .labeled_asset_scope(label, |_| SkinnedMeshInverseBindposes::from(inverse_bindposes));
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
                        loader_data,
                        Some(entity),
                        Some(&effects),
                        Some(&skinned_mesh),
                        Some(&net_nodes),
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

                if let Some(part_group) = self.nodes.get_as::<PartGroup>(node.child_refs[0] as usize) {
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
                } else {
                    warn!(name: "not_a_part_group", target: "Panda3DLoader",
                            "Tried to get node {}, but it wasn't a PartGroup. Unable to create joints, returning.", node.child_refs[0]);
                    return (inverse_bindposes, joints);
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
                    if let Some(node) = self.nodes.get_as::<ModelNode>(*net_node_ref as usize) {
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
                    } else {
                        warn!(name: "not_a_model_node", target: "Panda3DLoader",
                            "Tried to get node {} when trying to construct Net Transforms, but it wasn't a ModelNode, ignoring.", *net_node_ref);
                    }
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
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoadSettings {}

#[derive(Debug, Default)]
pub struct Panda3DLoader;

#[derive(Asset, TypePath, Debug, Default)]
pub struct Panda3DAsset {
    pub scene: Handle<Scene>,
    pub bindposes: Vec<Handle<SkinnedMeshInverseBindposes>>,
}

struct AssetLoaderData<'loader, 'context> {
    world: &'loader mut World,
    context: &'loader mut LoadContext<'context>,
    assets: &'loader mut Panda3DAsset,
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

            let mut loader_data = AssetLoaderData { world: &mut world, context, assets: &mut assets };

            block_on(bam.recurse_nodes(&mut loader_data, None, None, None, None, 0));

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
            .init_asset::<Panda3DAsset>()
            .add_plugins(MaterialPlugin::<
                ExtendedMaterial<StandardMaterial, Panda3DExtension>,
            >::default());
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
                depth_stencil.bias.constant = 1; //TODO: tweak these more if they give any trouble
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
