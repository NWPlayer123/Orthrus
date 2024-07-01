use crate::nodes::prelude::*;
use crate::prelude::BinaryAsset;

impl BinaryAsset {
    /// This function is used to recursively convert all child nodes
    pub(crate) fn recurse_nodes(&self, node_index: usize) {
        println!("Recurse {} {:?}\n", node_index, &self.nodes[node_index]);
        //TODO: has_decal, joint_map
        match &self.nodes[node_index] {
            PandaObject::ModelRoot(node) => {
                for child in &node.node.child_refs {
                    self.convert_node(child.0 as usize);
                }
            }
            PandaObject::ModelNode(node) => {
                for child in &node.node.child_refs {
                    self.convert_node(child.0 as usize);
                }
            }
            PandaObject::PandaNode(node) => {
                for child in &node.child_refs {
                    self.convert_node(child.0 as usize);
                }
            }
            PandaObject::GeomNode(node) => {
                for child in &node.node.child_refs {
                    self.convert_node(child.0 as usize);
                }
            }
            PandaObject::Character(node) => {
                for child in &node.node.node.child_refs {
                    self.convert_node(child.0 as usize);
                }
            }
            _ => (),
        }
    }

    /// This function is used to process node data and convert it to a useful format
    fn convert_node(&self, node_index: usize) {
        println!("{} {:?}", node_index, &self.nodes[node_index]);
        match &self.nodes[node_index] {
            PandaObject::GeomNode(_) => {
                self.convert_geom_node(node_index);
            }
            PandaObject::Character(_) => {
                self.convert_character_node(node_index);
            }
            _ => {
                // Otherwise, it's just a generic node and we need to keep recursing
                // TODO: register parent/child hierarchy?
                self.recurse_nodes(node_index)
            }
        }
    }

    fn convert_geom_node(&self, node_index: usize) {
        let node = match &self.nodes[node_index] {
            PandaObject::GeomNode(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        // TODO: handle node properties like billboard, transformstate, renderstate whenever they come up
        // TODO: whenever something actually has a decal, all this needs refactoring

        // (Geom, RenderState)
        for geom_ref in &node.geom_refs {
            let geom = match &self.nodes[geom_ref.0 as usize] {
                PandaObject::Geom(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            let render_state = match &self.nodes[geom_ref.1 as usize] {
                PandaObject::RenderState(node) => node,
                _ => panic!("Something has gone horribly wrong!"),
            };
            println!("{} {:?}", geom_ref.0, geom);
            println!("{} {:?}", geom_ref.1, render_state);
            for primitive_ref in &geom.primitive_refs {
                //TODO: get node and decompose it? We also should pass vertex_data
                self.convert_primitive(geom.data_ref as usize, *primitive_ref as usize);
            }
        }

        // After we've done our processing, check any child nodes
        self.recurse_nodes(node_index);
    }

    fn convert_primitive(&self, data_index: usize, node_index: usize) {
        let vertex_data = match &self.nodes[data_index] {
            PandaObject::GeomVertexData(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let vertex_format = match &self.nodes[vertex_data.format_ref as usize] {
            PandaObject::GeomVertexFormat(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        let primitive = match &self.nodes[node_index] {
            PandaObject::GeomTristrips(node) => node,
            PandaObject::GeomTriangles(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        println!("{} {:?}", data_index, vertex_data);
        println!("{} {:?}", vertex_data.format_ref, vertex_format);
        println!("{} {:?}", node_index, primitive);
    }

    fn convert_character_node(&self, node_index: usize) {
        let character = match &self.nodes[node_index] {
            PandaObject::Character(node) => node,
            _ => panic!("Something has gone horribly wrong!"),
        };
        //TODO: make group node
        //TODO: apply node properties
        self.recurse_nodes(node_index);

        for bundle_ref in &character.node.bundle_refs {
            self.convert_character_bundle(*bundle_ref as usize, None);
        }
    }

    fn convert_character_bundle(&self, node_index: usize, parent_joint: Option<&CharacterJoint>) {
        println!("{} {:?}", node_index, &self.nodes[node_index]);
        match &self.nodes[node_index] {
            PandaObject::CharacterJointBundle(node) => {
                for child_ref in &node.group.child_refs {
                    self.convert_character_bundle(*child_ref as usize, None);
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
                    self.convert_character_bundle(*child_ref as usize, Some(joint));
                }
            }
            PandaObject::PartGroup(node) => {
                for child_ref in &node.child_refs {
                    self.convert_character_bundle(*child_ref as usize, None);
                }
            }
            _ => panic!("Something has gone horribly wrong!"),
        }
    }
}
