use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("code must not be empty")]
    EmptyCode,

    #[error("branch name must not be empty")]
    EmptyBranchName,

    #[error("tag key must not be empty")]
    EmptyTagKey,

    #[error("tag value must not be empty")]
    EmptyTagValue,

    #[error("sdtmig version name must not be empty")]
    EmptySdtmigName,

    #[error("sdtm terminology version name must not be empty")]
    EmptySdtmTerminologyName,

    #[error("duplicate code in leaders: {0}")]
    DuplicateLeader(String),

    #[error("duplicate code in workers: {0}")]
    DuplicateWorker(String),

    #[error("unknown team type: {0}")]
    UnknownTeamType(String),

    #[error("unknown role type: {0}")]
    UnknownRoleType(String),

    #[error("unknown project language: {0}")]
    UnknownLanguage(String),

    #[error("not found")]
    NotFound,

    #[error("branch not found: {0}")]
    BranchNotFound(i32),

    #[error("branch project not found: {0}")]
    BranchProjectNotFound(i32),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("branch already exists: {0}")]
    DuplicateBranch(String),

    #[error("repository error: {0}")]
    Repository(String),
}
