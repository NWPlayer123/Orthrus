use super::prelude::*;

pub trait Node: core::fmt::Debug {
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error>
    where
        Self: Sized;
}

macro_rules! stored_types {
    ($($type:ident),+ $(,)?) => {
        paste::paste! {
            // Generate the type index enum
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum TypeIndex {
                $(
                    $type,
                )*
            }

            #[derive(Debug, Default)]
            pub struct NodeStorage {
                // Dense storage for each type
                $(
                    [<$type:snake>]: Vec<$type>,
                )*
                // Maps global ID -> (type, type-specific index)
                id_map: Vec<(TypeIndex, usize)>,
            }

            impl NodeStorage {
                /*pub fn new() -> Self {
                    Self {
                        $(
                            [<$type:snake>]: Vec::new(),
                        )*
                        id_map: Vec::new(),
                    }
                }*/

                #[allow(dead_code)]
                pub fn len(&self) -> usize {
                    self.id_map.len()
                }

                // Get an object's global ID
                pub fn push<T>(&mut self, node: T) -> usize
                where
                    T: StoredType
                {
                    let type_idx = T::type_index();
                    let local_idx = T::push_to_storage(self, node);
                    let global_idx = self.id_map.len();
                    self.id_map.push((type_idx, local_idx));
                    global_idx
                }

                // Get by global ID
                pub(crate) fn get(&self, id: usize) -> Option<NodeRef<'_>> {
                    let (type_idx, local_idx) = self.id_map.get(id)?;
                    Some(match type_idx {
                        $(
                            TypeIndex::$type => {
                                NodeRef::$type(self.[<$type:snake>].get(*local_idx)?)
                            }
                        )*
                    })
                }

                // Get typed reference if type matches
                pub fn get_as<T: StoredType>(&self, id: usize) -> Option<&T> {
                    let (type_idx, local_idx) = self.id_map.get(id)?;
                    if *type_idx == T::type_index() {
                        T::get_from_storage(self, *local_idx)
                    } else {
                        None
                    }
                }
            }

            // Enum for referencing any node type
            #[derive(Debug)]
            #[allow(dead_code)]
            pub(crate) enum NodeRef<'a> {
                $(
                    $type(&'a $type),
                )*
            }

            impl<'a> NodeRef<'a> {
                pub(crate) fn write_graph_data(&self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>) -> Result<(), bam::Error> {
                    match self {
                        $(
                            NodeRef::$type(node) => node.write_data(label, connections, true),
                        )*
                    }
                }
            }

            // Trait for stored types
            pub trait StoredType: Sized {
                fn type_index() -> TypeIndex;
                fn push_to_storage(storage: &mut NodeStorage, node: Self) -> usize;
                fn get_from_storage(storage: &NodeStorage, local_idx: usize) -> Option<&Self>;
            }

            // Implement for each type
            $(
                impl StoredType for $type {
                    fn type_index() -> TypeIndex {
                        TypeIndex::$type
                    }

                    fn push_to_storage(storage: &mut NodeStorage, node: Self) -> usize {
                        let idx = storage.[<$type:snake>].len();
                        storage.[<$type:snake>].push(node);
                        idx
                    }

                    fn get_from_storage(storage: &NodeStorage, local_idx: usize) -> Option<&Self> {
                        storage.[<$type:snake>].get(local_idx)
                    }
                }
            )*
        }
    }
}

stored_types!(
    AnimBundle,
    AnimBundleNode,
    AnimChannelMatrixXfmTable,
    AnimGroup,
    BillboardEffect,
    Character,
    CharacterJoint,
    CharacterJointEffect,
    CollisionCapsule,
    CollisionNode,
    CollisionPolygon,
    CollisionSphere,
    ColorAttrib,
    CullBinAttrib,
    CullFaceAttrib,
    DecalEffect,
    DepthWriteAttrib,
    Geom,
    GeomNode,
    GeomPrimitive,
    GeomVertexArrayData,
    GeomVertexArrayFormat,
    GeomVertexData,
    GeomVertexFormat,
    InternalName,
    JointVertexTransform,
    LODNode,
    ModelNode,
    PandaNode,
    PartBundle,
    PartGroup,
    RenderEffects,
    RenderState,
    SequenceNode,
    Texture,
    TextureAttrib,
    TextureStage,
    TransformBlendTable,
    TransformState,
    TransparencyAttrib,
    UserVertexTransform,
);
