use chrono::{DateTime, Utc};

use crate::domain::{
    DomainCategory, SdtmDomainDescription, SdtmProjectDomain, SdtmProjectDomainOverride,
    SdtmProjectVariable, SdtmProjectVariableOverride, SdtmProjectVersion, SdtmRole,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVersionView {
    pub id: i64,
    pub project_code: String,
    pub version_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectVersion> for SdtmProjectVersionView {
    fn from(v: SdtmProjectVersion) -> Self {
        Self {
            id: v.id,
            project_code: v.project_code,
            version_name: v.version_name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainView {
    pub id: i64,
    pub version_id: i64,
    pub domain_code: String,
    pub key_variables: Vec<String>,
    pub domain_override: Option<SdtmProjectDomainOverrideView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectDomain> for SdtmProjectDomainView {
    fn from(d: SdtmProjectDomain) -> Self {
        Self {
            id: d.id,
            version_id: d.version_id,
            domain_code: d.domain_code,
            key_variables: d.key_variables,
            domain_override: d.domain_override.map(Into::into),
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainOverrideView {
    pub id: i64,
    pub domain_id: i64,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectDomainOverride> for SdtmProjectDomainOverrideView {
    fn from(o: SdtmProjectDomainOverride) -> Self {
        Self {
            id: o.id,
            domain_id: o.domain_id,
            category: o.category,
            descriptions: o.descriptions,
            created_at: o.created_at,
            updated_at: o.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableView {
    pub id: i64,
    pub domain_id: i64,
    pub variable_code: String,
    pub variable_sequence: i64,
    pub variable_override: Option<SdtmProjectVariableOverrideView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectVariable> for SdtmProjectVariableView {
    fn from(v: SdtmProjectVariable) -> Self {
        Self {
            id: v.id,
            domain_id: v.domain_id,
            variable_code: v.variable_code,
            variable_sequence: v.variable_sequence,
            variable_override: v.variable_override.map(Into::into),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableOverrideView {
    pub id: i64,
    pub variable_id: i64,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectVariableOverride> for SdtmProjectVariableOverrideView {
    fn from(o: SdtmProjectVariableOverride) -> Self {
        Self {
            id: o.id,
            variable_id: o.variable_id,
            variable_type: o.variable_type,
            variable_core: o.variable_core,
            variable_role: o.variable_role,
            descriptions: o.descriptions,
            created_at: o.created_at,
            updated_at: o.updated_at,
        }
    }
}
