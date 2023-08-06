use core::fmt;

use compact_str::CompactString;
use hashbrown::HashMap;

pub trait Metadata: fmt::Display + bitflags::Flags {}

pub enum VirtualNode<T> {
    File(VirtualFile<T>),
    Folder(VirtualFolder<T>),
}

pub struct VirtualFile<T> {
    data: T,
}

#[derive(Default)]
pub struct VirtualFolder<T> {
    nodes: HashMap<CompactString, VirtualNode<T>>,
}

impl<T> VirtualFile<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> VirtualFolder<T> {
    pub fn insert(&mut self, path: &str, data: T) {
        let mut components = path.split('/').peekable();

        match components.next() {
            Some(component) => {
                // If we are not at the end of the path, insert into a subdirectory
                if components.peek().is_some() {
                    // Get the existing directory or create a new one
                    let directory =
                        self.nodes.entry(component.to_string().into()).or_insert_with(|| {
                            VirtualNode::Folder(Self {
                                //name: component.into(),
                                nodes: HashMap::new(),
                            })
                        });

                    // Recursively insert into the subdirectory
                    if let VirtualNode::Folder(ref mut directory) = directory {
                        directory
                            .insert(components.collect::<Vec<_>>().join("/").as_str(), data);
                    } else {
                        // Unexpected: this should be a directory, but it's a file
                        panic!("Path component is a file when it should be a directory");
                    }
                }
                // We are at the end of the path, insert the file
                else {
                    self.nodes.insert(
                        component.into(),
                        VirtualNode::File(VirtualFile { data }),
                    );
                }
            }
            None => {
                // Path is empty, nothing to insert
                panic!("Empty path");
            }
        }
    }

    pub fn debug_display(&self, indent: usize) {
        for (name, node) in self.nodes.iter() {
            let indentation = "  ".repeat(indent);
            match node {
                VirtualNode::File(file) => log::debug!("{}{}: {}", indentation, name, file),
                VirtualNode::Folder(folder) => {
                    log::debug!("{}{}: {{", indentation, name);
                    folder.debug_display(indent + 1);
                    log::debug!("{}}}", indentation);
                }
            }
        }
    }
}

impl<T> fmt::Display for VirtualFile<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File")
    }
}

impl<T> fmt::Display for VirtualFolder<T> {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_display(0);
        Ok(())
    }
}
