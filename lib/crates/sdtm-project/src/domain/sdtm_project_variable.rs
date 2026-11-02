use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::sdtm_project_variable_override::SdtmProjectVariableOverride;

/// A variable a project declares inside one of its domains.
/// `variable_sequence` is the column order within the parent domain.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmProjectVariable {
    pub id: i64,
    pub domain_id: i64,
    pub variable_code: String,
    pub variable_sequence: i64,
    pub variable_override: Option<SdtmProjectVariableOverride>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmProjectVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmProjectVariable")
            .field("id", &self.id)
            .field("domain_id", &self.domain_id)
            .field("variable_code", &self.variable_code)
            .field("variable_sequence", &self.variable_sequence)
            .field("variable_override", &self.variable_override)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmProjectVariable {
    /// Validating constructor used by the domain layer. Rejects an
    /// empty / whitespace `variable_code`.
    pub fn new(
        domain_id: i64,
        variable_code: String,
        variable_sequence: i64,
    ) -> Result<Self, DomainError> {
        if variable_code.trim().is_empty() {
            return Err(DomainError::EmptyVariableCode);
        }
        Ok(Self {
            id: 0,
            domain_id,
            variable_code,
            variable_sequence,
            variable_override: None,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        domain_id: i64,
        variable_code: String,
        variable_sequence: i64,
        variable_override: Option<SdtmProjectVariableOverride>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            domain_id,
            variable_code,
            variable_sequence,
            variable_override,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmProjectVariableRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmProjectVariableNew {
    pub domain_id: i64,
    pub variable_code: String,
    pub variable_sequence: i64,
}

/// Input DTO for `SdtmProjectVariableRepository::update`. `domain_id`
/// is immutable — a variable always belongs to the domain it was
/// created under.
#[derive(Debug, Clone, Default)]
pub struct SdtmProjectVariableUpdate {
    pub id: i64,
    pub variable_code: Option<String>,
    pub variable_sequence: Option<i64>,
}
