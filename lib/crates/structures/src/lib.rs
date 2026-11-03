//! # structures crate
//!
//! Workspace library that records the **directory structure** of a
//! clinical project: the set of folders and files that should exist on
//! disk, and the rule that lets a project *extend* a shared template of
//! that layout.
//!
//! The crate performs no filesystem I/O. It records what a structure
//! should look like; materialising it is the desktop app's job.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::internal::service::StructureServiceImpl;
pub use adapter::persistence::postgres::node_repo::NodeRepoPg;
pub use adapter::persistence::postgres::structure_repo::StructureRepoPg;
pub use adapter::service::project::project_lookup_impl::ProjectLookupImpl;

pub use domain::{
    DomainError, MergedNode, MergedTree, Migration, Node, NodeAttributes, NodeRepository,
    ProjectLookup, Structure, StructureFilter, StructureLevel, StructureNew, StructureRepository,
};
pub use usecase::{
    AddNode, CreateStructure, DeleteNode, MigrationView, NodeView, OwnNodeView, RenameStructure,
    StructureMetaView, StructureSummaryView, StructureTreeView, StructureUsecase,
    StructureUsecaseConfig, StructureView, UpdateNode, UsecaseError,
};
