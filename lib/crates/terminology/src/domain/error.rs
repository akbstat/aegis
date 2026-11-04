use thiserror::Error;

use super::terminology_kind::TerminologyKind;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("name must not be empty")]
    EmptyName,

    #[error("code must not be empty")]
    EmptyCode,

    #[error("invalid terminology kind: {0}")]
    InvalidKind(String),

    #[error("not found")]
    NotFound,

    #[error("version not found: {0}")]
    VersionNotFound(i64),

    #[error("code list not found: {0}")]
    CodeListNotFound(i64),

    #[error("code item not found: {0}")]
    CodeItemNotFound(i64),

    #[error("code at position {0} must not be empty")]
    EmptyCodeAtPosition(usize),

    /// Search fragment contained characters reserved by Postgres
    /// `to_tsquery` (`& | ! ( ) :`) or a quote character (`' "`).
    #[error("search fragment contains reserved tsquery characters: & | ! ( ) : ' \"")]
    InvalidFragment,

    #[error("terminology version already exists for {kind:?} / {name}")]
    DuplicateVersion { kind: TerminologyKind, name: String },

    #[error("code list already exists for version {version_id} / {code}")]
    DuplicateCodeList { version_id: i64, code: String },

    #[error("code item already exists for codelist {codelist_id} / {code}")]
    DuplicateCodeItem { codelist_id: i64, code: String },

    #[error("referenced terminology version not found: {0}")]
    FkVersionNotFound(i64),

    #[error("referenced code list not found: {0}")]
    FkCodeListNotFound(i64),

    #[error("repository error: {0}")]
    Repository(String),
}
