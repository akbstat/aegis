//! # structures crate
//!
//! Workspace library that records the **directory structure** of a
//! clinical project: the set of folders and files that should exist on
//! disk, and the rule that lets a project *extend* a shared template of
//! that layout.
//!
//! The crate performs no filesystem I/O. It records what a structure
//! should look like; materialising it is the desktop app's job.

pub mod domain;

pub use domain::{
    DomainError, MergedNode, MergedTree, Migration, Node, NodeAttributes, StructureLevel,
};
