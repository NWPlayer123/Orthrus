use crate::nodes::prelude::*;
use crate::prelude::*;

use bevy_asset::io::Reader;
use bevy_asset::{Asset, AssetLoader, Handle};
use bevy_reflect::TypePath;
use bevy_scene::Scene;

#[derive(Asset, Debug, Default, TypePath)]
pub struct BinaryAsset {
    pub scenes: Vec<Handle<Scene>>,
}

impl BinaryAsset {
    /// This function is used to recursively convert all child nodes
    pub(crate) fn recurse_nodes(&self, asset: &crate::bam::BinaryAsset, node_index: usize) {
        println!("Recurse {} {:?}\n", node_index, &asset.nodes[node_index]);
        //TODO: has_decal, joint_map
        match &asset.nodes[node_index] {
            PandaObject::ModelRoot(node) => {
                for child in &node.node.child_refs {
                    self.convert_node(asset, child.0 as usize);
                }
            }
            PandaObject::ModelNode(node) => {
                for child in &node.node.child_refs {
                    self.convert_node(asset, child.0 as usize);
                }
            }
            PandaObject::PandaNode(node) => {
                for child in &node.child_refs {
                    self.convert_node(asset, child.0 as usize);
                }
            }
            PandaObject::GeomNode(node) => {
                for child in &node.node.child_refs {
                    self.convert_node(asset, child.0 as usize);
                }
            }
            PandaObject::Character(node) => {
                for child in &node.node.node.child_refs {
                    self.convert_node(asset, child.0 as usize);
                }
            }
            _ => (),
        }
    }

    /// This function is used to process node data and convert it to a useful format
    fn convert_node(&self, asset: &crate::bam::BinaryAsset, node_index: usize) {
        println!("{} {:?}", node_index, &asset.nodes[node_index]);
        match &asset.nodes[node_index] {
            PandaObject::GeomNode(_) => {
                self.convert_geom_node(asset, node_index);
            }
            PandaObject::Character(_) => {
                self.convert_character_node(asset, node_index);
            }
            _ => {
                // Otherwise, it's just a generic node and we need to keep recursing
                // TODO: register parent/child hierarchy?
                self.recurse_nodes(asset, node_index)
            }
        }
    }

    fn convert_geom_node(&self, asset: &crate::bam::BinaryAsset, node_index: usize) {
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
                self.convert_primitive(asset, geom.data_ref as usize, *primitive_ref as usize);
            }
        }

        // After we've done our processing, check any child nodes
        self.recurse_nodes(asset, node_index);
    }

    fn convert_primitive(&self, asset: &crate::bam::BinaryAsset, data_index: usize, node_index: usize) {
        let vertex_data = match &asset.nodes[data_index] {
            PandaObject::GeomVertexData(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let vertex_format = match &asset.nodes[vertex_data.format_ref as usize] {
            PandaObject::GeomVertexFormat(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let primitive = match &asset.nodes[node_index] {
            PandaObject::GeomTristrips(node) => node,
            PandaObject::GeomTriangles(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        println!("{} {:?}", data_index, vertex_data);
        println!("{} {:?}", vertex_data.format_ref, vertex_format);
        println!("{} {:?}", node_index, primitive);
    }

    fn convert_character_node(&self, asset: &crate::bam::BinaryAsset, node_index: usize) {
        let character = match &asset.nodes[node_index] {
            PandaObject::Character(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        //TODO: make group node
        //TODO: apply node properties
        self.recurse_nodes(asset, node_index);

        for bundle_ref in &character.node.bundle_refs {
            self.convert_character_bundle(asset,*bundle_ref as usize, None);
        }
    }

    fn convert_character_bundle(&self, asset: &crate::bam::BinaryAsset, node_index: usize, parent_joint: Option<&CharacterJoint>) {
        println!("{} {:?}", node_index, &asset.nodes[node_index]);
        match &asset.nodes[node_index] {
            PandaObject::CharacterJointBundle(node) => {
                for child_ref in &node.group.child_refs {
                    self.convert_character_bundle(asset, *child_ref as usize, None);
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
                    self.convert_character_bundle(asset, *child_ref as usize, Some(joint));
                }
            }
            PandaObject::PartGroup(node) => {
                for child_ref in &node.child_refs {
                    self.convert_character_bundle(asset, *child_ref as usize, None);
                }
            }
            _ => panic!("Something has gone horribly wrong!"),
        }
    }
}

#[derive(Default)]
struct BamAssetLoader;

impl AssetLoader for BamAssetLoader {
    type Asset = BinaryAsset;
    type Error = bam::Error;
    type Settings = ();

    async fn load<'a>(
        &'a self, reader: &'a mut dyn Reader, _settings: &'a Self::Settings,
        _load_context: &'a mut bevy_asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // First, load the actual data into something we can pass around
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Then, parse the actual BAM file
        let bam = crate::bam::BinaryAsset::load(bytes)?;

        // Finally, we can actually generate the data
        let asset = BinaryAsset::default();
        asset.recurse_nodes(&bam, 1);
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["bam"]
    }
}
