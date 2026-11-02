mod error;
mod merge;
mod migration;
mod node;
mod structure_level;

#[cfg(test)]
mod tests;

pub use error::DomainError;
pub use merge::{merge, MergedNode, MergedTree};
pub use migration::Migration;
pub use node::{Node, NodeAttributes};
pub use structure_level::StructureLevel;
