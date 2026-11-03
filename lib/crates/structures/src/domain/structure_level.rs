use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Which kind of structure this is.
///
/// `Template` structures are global and shared; `Project` structures
/// belong to one project and extend exactly one template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureLevel {
    Template,
    Project,
}

impl StructureLevel {
    /// The stable wire code. New levels land here as new variants.
    pub fn as_str(&self) -> &'static str {
        match self {
            StructureLevel::Template => "template",
            StructureLevel::Project => "project",
        }
    }
}

impl TryFrom<&str> for StructureLevel {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "template" => Ok(StructureLevel::Template),
            "project" => Ok(StructureLevel::Project),
            other => Err(DomainError::InvalidStructureLevel(other.to_string())),
        }
    }
}
