//! HTTP error mapping.
//!
//! [`ApiError`] is an enum that wraps every apis error type the HTTP
//! layer surfaces today — [`apis::auth::AuthApiError`] and
//! [`apis::user::UserApiError`]. Every handler returns
//! `Result<Json<T>, ApiError>` and uses `?` on either inner error; the
//! [`From`] impls (derived via `#[from]`) do the wrapping.
//!
//! New apis services land as additional enum variants; the `status()`
//! and `code()` dispatch tables pick them up.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// Stable JSON error envelope returned to clients.
///
/// `code` is a machine-readable string (e.g. `invalid_credentials`)
/// that clients should switch on. `message` is human-readable and
/// may be surfaced in a UI.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

/// Error type returned by every HTTP handler.
///
/// Each variant wraps an apis-level error and implements
/// [`IntoResponse`] so handlers can return `Result<_, ApiError>` and
/// let the `?` operator do the conversion. The dispatch tables live
/// in private `*_status` / `*_code` helpers so the public `status()`
/// and `code()` methods stay table-shaped.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    Auth(#[from] apis::auth::AuthApiError),

    #[error("{0}")]
    User(#[from] apis::user::UserApiError),

    #[error("{0}")]
    Project(#[from] apis::project::ProjectApiError),

    #[error("{0}")]
    Terminology(#[from] apis::terminology::TerminologyApiError),

    #[error("{0}")]
    DomainModel(#[from] apis::domain_model::DomainModelApiError),

    #[error("{0}")]
    Crf(#[from] apis::crf::CrfApiError),

    #[error("{0}")]
    Mission(#[from] apis::mission::MissionApiError),

    #[error("{0}")]
    SdtmProject(#[from] apis::sdtm_project::SdtmProjectApiError),

    #[error("{0}")]
    Structures(#[from] apis::structures::StructureApiError),

    #[error("{0}")]
    LogIngest(#[from] logging_utils::IngestorError),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("admin or root role required")]
    Forbidden,
}

impl ApiError {
    /// HTTP status code for this error variant.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Auth(e) => auth_status(e),
            Self::User(e) => user_status(e),
            Self::Project(e) => project_status(e),
            Self::Terminology(e) => terminology_status(e),
            Self::DomainModel(e) => domain_model_status(e),
            Self::Crf(e) => crf_status(e),
            Self::SdtmProject(e) => sdtm_project_status(e),
            Self::Structures(e) => structures_status(e),
            Self::Mission(e) => mission_status(e),
            Self::LogIngest(e) => log_ingest_status(e),
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Forbidden => StatusCode::FORBIDDEN,
        }
    }

    /// Stable machine-readable code used as `ErrorBody.code`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Auth(e) => auth_code(e),
            Self::User(e) => user_code(e),
            Self::Project(e) => project_code(e),
            Self::Terminology(e) => terminology_code(e),
            Self::DomainModel(e) => domain_model_code(e),
            Self::Crf(e) => crf_code(e),
            Self::SdtmProject(e) => sdtm_project_code(e),
            Self::Structures(e) => structures_code(e),
            Self::Mission(e) => mission_code(e),
            Self::LogIngest(e) => log_ingest_code(e),
            Self::Validation(_) => "validation_failed",
            Self::Forbidden => "forbidden",
        }
    }
}

fn auth_status(e: &apis::auth::AuthApiError) -> StatusCode {
    use apis::auth::AuthApiError;
    match e {
        AuthApiError::Validation(_) => StatusCode::BAD_REQUEST,
        AuthApiError::NotFound => StatusCode::NOT_FOUND,
        AuthApiError::Inactive => StatusCode::FORBIDDEN,
        AuthApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        AuthApiError::Verification(_) => StatusCode::UNAUTHORIZED,
        AuthApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        AuthApiError::Signing(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AuthApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn auth_code(e: &apis::auth::AuthApiError) -> &'static str {
    use apis::auth::AuthApiError;
    match e {
        AuthApiError::Validation(_) => "validation_failed",
        AuthApiError::NotFound => "not_found",
        AuthApiError::Inactive => "user_inactive",
        AuthApiError::InvalidCredentials => "invalid_credentials",
        AuthApiError::Verification(_) => "token_verification_failed",
        AuthApiError::DuplicateCode(_) => "duplicate_code",
        AuthApiError::Signing(_) => "signing_failed",
        AuthApiError::Repository(_) => "repository_error",
    }
}

fn user_status(e: &apis::user::UserApiError) -> StatusCode {
    use apis::user::UserApiError;
    match e {
        UserApiError::Validation(_) => StatusCode::BAD_REQUEST,
        UserApiError::NotFound => StatusCode::NOT_FOUND,
        UserApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        UserApiError::Hashing(_) => StatusCode::INTERNAL_SERVER_ERROR,
        UserApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn user_code(e: &apis::user::UserApiError) -> &'static str {
    use apis::user::UserApiError;
    match e {
        UserApiError::Validation(_) => "validation_failed",
        UserApiError::NotFound => "not_found",
        UserApiError::DuplicateCode(_) => "duplicate_code",
        UserApiError::Hashing(_) => "hashing_failed",
        UserApiError::Repository(_) => "repository_error",
    }
}

fn project_status(e: &apis::project::ProjectApiError) -> StatusCode {
    use apis::project::ProjectApiError;
    match e {
        ProjectApiError::Validation(_) => StatusCode::BAD_REQUEST,
        ProjectApiError::NotFound | ProjectApiError::UserNotFound(_) => StatusCode::NOT_FOUND,
        ProjectApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        ProjectApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn project_code(e: &apis::project::ProjectApiError) -> &'static str {
    use apis::project::ProjectApiError;
    match e {
        ProjectApiError::Validation(_) => "validation_failed",
        ProjectApiError::NotFound => "not_found",
        ProjectApiError::UserNotFound(_) => "user_not_found",
        ProjectApiError::DuplicateCode(_) => "duplicate_code",
        ProjectApiError::Repository(_) => "repository_error",
    }
}

fn terminology_status(e: &apis::terminology::TerminologyApiError) -> StatusCode {
    use apis::terminology::TerminologyApiError;
    match e {
        TerminologyApiError::Validation(_) => StatusCode::BAD_REQUEST,
        TerminologyApiError::NotFound => StatusCode::NOT_FOUND,
        TerminologyApiError::DuplicateVersion { .. }
        | TerminologyApiError::DuplicateCodeList { .. }
        | TerminologyApiError::DuplicateCodeItem { .. } => StatusCode::CONFLICT,
        TerminologyApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn terminology_code(e: &apis::terminology::TerminologyApiError) -> &'static str {
    use apis::terminology::TerminologyApiError;
    match e {
        TerminologyApiError::Validation(_) => "validation_failed",
        TerminologyApiError::NotFound => "not_found",
        TerminologyApiError::DuplicateVersion { .. } => "duplicate_terminology_version",
        TerminologyApiError::DuplicateCodeList { .. } => "duplicate_code_list",
        TerminologyApiError::DuplicateCodeItem { .. } => "duplicate_code_item",
        TerminologyApiError::Repository(_) => "repository_error",
    }
}

fn domain_model_status(e: &apis::domain_model::DomainModelApiError) -> StatusCode {
    use apis::domain_model::DomainModelApiError;
    match e {
        DomainModelApiError::Validation(_) => StatusCode::BAD_REQUEST,
        DomainModelApiError::NotFound
        | DomainModelApiError::SdtmVersionNotFound(_)
        | DomainModelApiError::SdtmDomainNotFound(_)
        | DomainModelApiError::SdtmVariableNotFound(_) => StatusCode::NOT_FOUND,
        DomainModelApiError::DuplicateSdtmVersion { .. }
        | DomainModelApiError::DuplicateSdtmDomain { .. }
        | DomainModelApiError::DuplicateSdtmVariable { .. } => StatusCode::CONFLICT,
        DomainModelApiError::FkSdtmVersionNotFound(_)
        | DomainModelApiError::FkSdtmDomainNotFound(_) => StatusCode::BAD_REQUEST,
        DomainModelApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn domain_model_code(e: &apis::domain_model::DomainModelApiError) -> &'static str {
    use apis::domain_model::DomainModelApiError;
    match e {
        DomainModelApiError::Validation(_) => "validation_failed",
        DomainModelApiError::NotFound => "not_found",
        DomainModelApiError::SdtmVersionNotFound(_) => "sdtm_version_not_found",
        DomainModelApiError::SdtmDomainNotFound(_) => "sdtm_domain_not_found",
        DomainModelApiError::SdtmVariableNotFound(_) => "sdtm_variable_not_found",
        DomainModelApiError::DuplicateSdtmVersion { .. } => "duplicate_sdtm_version",
        DomainModelApiError::DuplicateSdtmDomain { .. } => "duplicate_sdtm_domain",
        DomainModelApiError::DuplicateSdtmVariable { .. } => "duplicate_sdtm_variable",
        DomainModelApiError::FkSdtmVersionNotFound(_) => "fk_sdtm_version_not_found",
        DomainModelApiError::FkSdtmDomainNotFound(_) => "fk_sdtm_domain_not_found",
        DomainModelApiError::Repository(_) => "repository_error",
    }
}

fn structures_status(e: &apis::structures::StructureApiError) -> StatusCode {
    use apis::structures::StructureApiError;
    match e {
        StructureApiError::Validation(_) => StatusCode::BAD_REQUEST,
        StructureApiError::NotFound
        | StructureApiError::StructureNotFound(_)
        | StructureApiError::NodeNotFound(_)
        | StructureApiError::TemplateNotFound(_)
        | StructureApiError::ProjectNotFound(_) => StatusCode::NOT_FOUND,
        StructureApiError::DuplicateName(_)
        | StructureApiError::DuplicateProjectStructure(_)
        | StructureApiError::TemplateInUse(_) => StatusCode::CONFLICT,
        StructureApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn structures_code(e: &apis::structures::StructureApiError) -> &'static str {
    use apis::structures::StructureApiError;
    match e {
        StructureApiError::Validation(_) => "validation_failed",
        StructureApiError::NotFound => "not_found",
        StructureApiError::StructureNotFound(_) => "structure_not_found",
        StructureApiError::NodeNotFound(_) => "node_not_found",
        StructureApiError::TemplateNotFound(_) => "template_not_found",
        StructureApiError::ProjectNotFound(_) => "project_not_found",
        StructureApiError::DuplicateName(_) => "duplicate_name",
        StructureApiError::DuplicateProjectStructure(_) => "duplicate_project_structure",
        StructureApiError::TemplateInUse(_) => "template_in_use",
        StructureApiError::Repository(_) => "repository_error",
    }
}

fn sdtm_project_status(e: &apis::sdtm_project::SdtmProjectApiError) -> StatusCode {
    use apis::sdtm_project::SdtmProjectApiError;
    match e {
        SdtmProjectApiError::Validation(_) => StatusCode::BAD_REQUEST,
        // A missing parent is a 404, not a 400: the client named a
        // resource that is not there.
        SdtmProjectApiError::NotFound
        | SdtmProjectApiError::ProjectNotFound(_)
        | SdtmProjectApiError::SdtmProjectVersionNotFound(_)
        | SdtmProjectApiError::SdtmProjectDomainNotFound(_)
        | SdtmProjectApiError::SdtmProjectDomainOverrideNotFound(_)
        | SdtmProjectApiError::SdtmProjectVariableNotFound(_)
        | SdtmProjectApiError::SdtmProjectVariableOverrideNotFound(_)
        | SdtmProjectApiError::FkSdtmProjectVersionNotFound(_)
        | SdtmProjectApiError::FkSdtmProjectDomainNotFound(_)
        | SdtmProjectApiError::FkSdtmProjectVariableNotFound(_) => StatusCode::NOT_FOUND,
        SdtmProjectApiError::DuplicateSdtmProjectVersion { .. }
        | SdtmProjectApiError::DuplicateSdtmProjectDomain { .. }
        | SdtmProjectApiError::DuplicateSdtmProjectDomainOverride { .. }
        | SdtmProjectApiError::DuplicateSdtmProjectVariable { .. }
        | SdtmProjectApiError::DuplicateSdtmProjectVariableOverride { .. } => StatusCode::CONFLICT,
        SdtmProjectApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn sdtm_project_code(e: &apis::sdtm_project::SdtmProjectApiError) -> &'static str {
    use apis::sdtm_project::SdtmProjectApiError;
    match e {
        SdtmProjectApiError::Validation(_) => "validation_failed",
        SdtmProjectApiError::NotFound => "not_found",
        SdtmProjectApiError::ProjectNotFound(_) => "project_not_found",
        SdtmProjectApiError::SdtmProjectVersionNotFound(_) => "sdtm_project_version_not_found",
        SdtmProjectApiError::SdtmProjectDomainNotFound(_) => "sdtm_project_domain_not_found",
        SdtmProjectApiError::SdtmProjectDomainOverrideNotFound(_) => {
            "sdtm_project_domain_override_not_found"
        }
        SdtmProjectApiError::SdtmProjectVariableNotFound(_) => "sdtm_project_variable_not_found",
        SdtmProjectApiError::SdtmProjectVariableOverrideNotFound(_) => {
            "sdtm_project_variable_override_not_found"
        }
        SdtmProjectApiError::DuplicateSdtmProjectVersion { .. } => "duplicate_sdtm_project_version",
        SdtmProjectApiError::DuplicateSdtmProjectDomain { .. } => "duplicate_sdtm_project_domain",
        SdtmProjectApiError::DuplicateSdtmProjectDomainOverride { .. } => {
            "duplicate_sdtm_project_domain_override"
        }
        SdtmProjectApiError::DuplicateSdtmProjectVariable { .. } => {
            "duplicate_sdtm_project_variable"
        }
        SdtmProjectApiError::DuplicateSdtmProjectVariableOverride { .. } => {
            "duplicate_sdtm_project_variable_override"
        }
        SdtmProjectApiError::FkSdtmProjectVersionNotFound(_) => "fk_sdtm_project_version_not_found",
        SdtmProjectApiError::FkSdtmProjectDomainNotFound(_) => "fk_sdtm_project_domain_not_found",
        SdtmProjectApiError::FkSdtmProjectVariableNotFound(_) => {
            "fk_sdtm_project_variable_not_found"
        }
        SdtmProjectApiError::Repository(_) => "repository_error",
    }
}

fn crf_status(e: &apis::crf::CrfApiError) -> StatusCode {
    use apis::crf::CrfApiError;
    match e {
        CrfApiError::Validation(_) | CrfApiError::EmptySearchFragment => StatusCode::BAD_REQUEST,
        CrfApiError::NotFound
        | CrfApiError::ProjectNotFound(_)
        | CrfApiError::CrfVersionNotFound(_)
        | CrfApiError::CrfFormNotFound(_)
        | CrfApiError::CrfItemNotFound(_)
        | CrfApiError::CrfOptionNotFound(_)
        | CrfApiError::CrfUnitNotFound(_)
        | CrfApiError::DomainAnnotationNotFound(_)
        | CrfApiError::AnnotationNotFound(_) => StatusCode::NOT_FOUND,
        CrfApiError::DuplicateCrfVersion { .. }
        | CrfApiError::DuplicateCrfForm { .. }
        | CrfApiError::DuplicateCrfItem { .. }
        | CrfApiError::DuplicateDomainAnnotation { .. } => StatusCode::CONFLICT,
        CrfApiError::KindShapeViolation { .. } => StatusCode::BAD_REQUEST,
        CrfApiError::FkCrfVersionNotFound(_)
        | CrfApiError::FkCrfFormNotFound(_)
        | CrfApiError::FkCrfItemNotFound(_)
        | CrfApiError::FkCrfOptionNotFound(_)
        | CrfApiError::FkCrfUnitNotFound(_)
        | CrfApiError::FkDomainAnnotationNotFound(_) => StatusCode::BAD_REQUEST,
        CrfApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn crf_code(e: &apis::crf::CrfApiError) -> &'static str {
    use apis::crf::CrfApiError;
    match e {
        CrfApiError::Validation(_) => "validation_failed",
        CrfApiError::NotFound => "not_found",
        CrfApiError::ProjectNotFound(_) => "project_not_found",
        CrfApiError::CrfVersionNotFound(_) => "crf_version_not_found",
        CrfApiError::CrfFormNotFound(_) => "crf_form_not_found",
        CrfApiError::CrfItemNotFound(_) => "crf_item_not_found",
        CrfApiError::CrfOptionNotFound(_) => "crf_option_not_found",
        CrfApiError::CrfUnitNotFound(_) => "crf_unit_not_found",
        CrfApiError::DomainAnnotationNotFound(_) => "domain_annotation_not_found",
        CrfApiError::AnnotationNotFound(_) => "annotation_not_found",
        CrfApiError::DuplicateCrfVersion { .. } => "duplicate_crf_version",
        CrfApiError::DuplicateCrfForm { .. } => "duplicate_crf_form",
        CrfApiError::DuplicateCrfItem { .. } => "duplicate_crf_item",
        CrfApiError::DuplicateDomainAnnotation { .. } => "duplicate_domain_annotation",
        CrfApiError::FkCrfVersionNotFound(_) => "fk_crf_version_not_found",
        CrfApiError::FkCrfFormNotFound(_) => "fk_crf_form_not_found",
        CrfApiError::FkCrfItemNotFound(_) => "fk_crf_item_not_found",
        CrfApiError::FkCrfOptionNotFound(_) => "fk_crf_option_not_found",
        CrfApiError::FkCrfUnitNotFound(_) => "fk_crf_unit_not_found",
        CrfApiError::FkDomainAnnotationNotFound(_) => "fk_domain_annotation_not_found",
        CrfApiError::EmptySearchFragment => "empty_search_fragment",
        CrfApiError::KindShapeViolation { .. } => "kind_shape_violation",
        CrfApiError::Repository(_) => "repository_error",
    }
}

fn mission_status(e: &apis::mission::MissionApiError) -> StatusCode {
    use apis::mission::MissionApiError;
    match e {
        MissionApiError::Validation(_) => StatusCode::BAD_REQUEST,
        MissionApiError::NotFound
        | MissionApiError::AssigneeNotFound
        | MissionApiError::IssueNotFound
        | MissionApiError::MissionNotFoundForIssue(_) => StatusCode::NOT_FOUND,
        MissionApiError::ProjectNotFound(_) | MissionApiError::UserNotFound(_) => {
            StatusCode::NOT_FOUND
        }
        MissionApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
        MissionApiError::DuplicateMission { .. } | MissionApiError::DuplicateAssignee { .. } => {
            StatusCode::CONFLICT
        }
        MissionApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn mission_code(e: &apis::mission::MissionApiError) -> &'static str {
    use apis::mission::MissionApiError;
    match e {
        MissionApiError::Validation(_) => "validation_failed",
        MissionApiError::NotFound => "not_found",
        MissionApiError::AssigneeNotFound => "assignee_not_found",
        MissionApiError::IssueNotFound => "issue_not_found",
        MissionApiError::MissionNotFoundForIssue(_) => "mission_not_found_for_issue",
        MissionApiError::ProjectNotFound(_) => "project_not_found",
        MissionApiError::UserNotFound(_) => "user_not_found",
        MissionApiError::Forbidden { .. } => "forbidden",
        MissionApiError::DuplicateMission { .. } => "duplicate_mission",
        MissionApiError::DuplicateAssignee { .. } => "duplicate_assignee",
        MissionApiError::Repository(_) => "repository_error",
    }
}

fn log_ingest_status(e: &logging_utils::IngestorError) -> StatusCode {
    use logging_utils::IngestorError;
    match e {
        IngestorError::DuplicateBatchId(_) => StatusCode::CONFLICT,
        IngestorError::ChannelFull(_) | IngestorError::ShutDown => StatusCode::SERVICE_UNAVAILABLE,
        IngestorError::CreateDir { .. }
        | IngestorError::WriterJoinTimeout { .. }
        | IngestorError::WriterPanic(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn log_ingest_code(e: &logging_utils::IngestorError) -> &'static str {
    use logging_utils::IngestorError;
    match e {
        IngestorError::DuplicateBatchId(_) => "duplicate_batch_id",
        IngestorError::ChannelFull(_) => "channel_full",
        IngestorError::ShutDown => "ingestor_shut_down",
        IngestorError::CreateDir { .. } => "log_dir_create_failed",
        IngestorError::WriterJoinTimeout { .. } => "writer_join_timeout",
        IngestorError::WriterPanic(_) => "writer_panic",
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(
                code = self.code(),
                error = %self,
                "api error",
            );
        }
        let body = ErrorBody {
            code: self.code().to_string(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `IntoResponse::into_response` and recover the status +
    /// JSON body so each `AuthApiError` variant can be asserted
    /// directly. The body bytes are re-parsed into `ErrorBody` for a
    /// structured comparison.
    async fn render(err: apis::auth::AuthApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    /// Drive `IntoResponse::into_response` for a `UserApiError` and
    /// recover the status + JSON body so each variant can be
    /// asserted directly.
    async fn render_user(err: apis::user::UserApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    // ---- AuthApiError mapping (unchanged from prior behaviour) -----

    #[tokio::test]
    async fn validation_maps_to_400() {
        let (status, body) = render(apis::auth::AuthApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
        assert_eq!(body.message, "validation failed: bad");
    }

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let (status, body) = render(apis::auth::AuthApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn inactive_maps_to_403() {
        let (status, body) = render(apis::auth::AuthApiError::Inactive).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.code, "user_inactive");
    }

    #[tokio::test]
    async fn invalid_credentials_maps_to_401() {
        let (status, body) = render(apis::auth::AuthApiError::InvalidCredentials).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "invalid_credentials");
    }

    #[tokio::test]
    async fn verification_maps_to_401() {
        let (status, body) = render(apis::auth::AuthApiError::Verification("bad sig".into())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "token_verification_failed");
    }

    #[tokio::test]
    async fn duplicate_code_maps_to_409() {
        let (status, body) = render(apis::auth::AuthApiError::DuplicateCode("u1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn signing_maps_to_500() {
        let (status, body) = render(apis::auth::AuthApiError::Signing("boom".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "signing_failed");
    }

    #[tokio::test]
    async fn repository_maps_to_500() {
        let (status, body) = render(apis::auth::AuthApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    #[test]
    fn from_auth_api_error_wraps() {
        let inner = apis::auth::AuthApiError::NotFound;
        let outer = ApiError::from(inner);
        // `AuthApiError` does not implement `PartialEq`, so assert
        // through the rendering path which is stable for these
        // unit-style variants.
        assert_eq!(outer.status(), StatusCode::NOT_FOUND);
        assert_eq!(outer.code(), "not_found");
    }

    // ---- UserApiError mapping (new) -----

    #[tokio::test]
    async fn user_validation_maps_to_400() {
        let (status, body) = render_user(apis::user::UserApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
        assert_eq!(body.message, "validation failed: bad");
    }

    #[tokio::test]
    async fn user_not_found_maps_to_404() {
        let (status, body) = render_user(apis::user::UserApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn user_duplicate_code_maps_to_409() {
        let (status, body) =
            render_user(apis::user::UserApiError::DuplicateCode("u1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn user_hashing_maps_to_500() {
        let (status, body) = render_user(apis::user::UserApiError::Hashing("oops".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "hashing_failed");
    }

    #[tokio::test]
    async fn user_repository_maps_to_500() {
        let (status, body) =
            render_user(apis::user::UserApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- ProjectApiError mapping -----

    async fn render_project(err: apis::project::ProjectApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    #[tokio::test]
    async fn project_validation_maps_to_400() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn project_not_found_maps_to_404() {
        let (status, body) = render_project(apis::project::ProjectApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn project_user_not_found_maps_to_404() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::UserNotFound("u1".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "user_not_found");
    }

    #[tokio::test]
    async fn project_duplicate_code_maps_to_409() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::DuplicateCode("dup".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn project_repository_maps_to_500() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- Forbidden (authorization) -----

    #[tokio::test]
    async fn forbidden_maps_to_403() {
        let response = ApiError::Forbidden.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.code, "forbidden");
        assert_eq!(parsed.message, "admin or root role required");
    }

    // ---- TerminologyApiError mapping -----

    async fn render_terminology(
        err: apis::terminology::TerminologyApiError,
    ) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    #[tokio::test]
    async fn terminology_validation_maps_to_400() {
        let (status, body) = render_terminology(
            apis::terminology::TerminologyApiError::Validation("bad".into()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn terminology_not_found_maps_to_404() {
        let (status, body) =
            render_terminology(apis::terminology::TerminologyApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn terminology_duplicate_version_maps_to_409() {
        let (status, body) =
            render_terminology(apis::terminology::TerminologyApiError::DuplicateVersion {
                kind: apis::terminology::TerminologyKind::Sdtm,
                name: "v1".into(),
            })
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_terminology_version");
    }

    #[tokio::test]
    async fn terminology_duplicate_code_list_maps_to_409() {
        let (status, body) =
            render_terminology(apis::terminology::TerminologyApiError::DuplicateCodeList {
                version_id: 1,
                code: "C66741".into(),
            })
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code_list");
    }

    #[tokio::test]
    async fn terminology_duplicate_code_item_maps_to_409() {
        let (status, body) =
            render_terminology(apis::terminology::TerminologyApiError::DuplicateCodeItem {
                codelist_id: 1,
                code: "C1".into(),
            })
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code_item");
    }

    #[tokio::test]
    async fn terminology_repository_maps_to_500() {
        let (status, body) = render_terminology(
            apis::terminology::TerminologyApiError::Repository("db".into()),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- DomainModelApiError mapping -----

    async fn render_domain_model(
        err: apis::domain_model::DomainModelApiError,
    ) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    #[tokio::test]
    async fn domain_model_validation_maps_to_400() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::Validation("bad".into()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn domain_model_not_found_maps_to_404() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn domain_model_sdtm_version_not_found_maps_to_404() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::SdtmVersionNotFound(42))
                .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "sdtm_version_not_found");
    }

    #[tokio::test]
    async fn domain_model_sdtm_domain_not_found_maps_to_404() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::SdtmDomainNotFound(42),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "sdtm_domain_not_found");
    }

    #[tokio::test]
    async fn domain_model_sdtm_variable_not_found_maps_to_404() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::SdtmVariableNotFound(42))
                .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "sdtm_variable_not_found");
    }

    #[tokio::test]
    async fn domain_model_duplicate_sdtm_version_maps_to_409() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::DuplicateSdtmVersion { name: "v1".into() },
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_sdtm_version");
    }

    #[tokio::test]
    async fn domain_model_duplicate_sdtm_domain_maps_to_409() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::DuplicateSdtmDomain {
                version_id: 1,
                name: "DM".into(),
            },
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_sdtm_domain");
    }

    #[tokio::test]
    async fn domain_model_duplicate_sdtm_variable_maps_to_409() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::DuplicateSdtmVariable {
                domain_id: 1,
                name: "AGE".into(),
            },
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_sdtm_variable");
    }

    #[tokio::test]
    async fn domain_model_fk_sdtm_version_not_found_maps_to_400() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::FkSdtmVersionNotFound(99))
                .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "fk_sdtm_version_not_found");
    }

    #[tokio::test]
    async fn domain_model_fk_sdtm_domain_not_found_maps_to_400() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::FkSdtmDomainNotFound(99))
                .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "fk_sdtm_domain_not_found");
    }

    #[tokio::test]
    async fn domain_model_repository_maps_to_500() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::Repository("db".into()),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- CrfApiError mapping -----

    /// Drive `IntoResponse::into_response` for a
    /// `SdtmProjectApiError` and recover the status + JSON body so
    /// each variant can be asserted directly.
    async fn render_sdtm_project(
        err: apis::sdtm_project::SdtmProjectApiError,
    ) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    #[tokio::test]
    async fn sdtm_project_validation_maps_to_400() {
        let (status, body) = render_sdtm_project(
            apis::sdtm_project::SdtmProjectApiError::Validation("bad".into()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn sdtm_project_missing_version_maps_to_404() {
        let (status, body) = render_sdtm_project(
            apis::sdtm_project::SdtmProjectApiError::SdtmProjectVersionNotFound(7),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "sdtm_project_version_not_found");
    }

    #[tokio::test]
    async fn sdtm_project_missing_parent_maps_to_404() {
        // A missing parent is a 404: the client named a resource that
        // is not there.
        let (status, body) = render_sdtm_project(
            apis::sdtm_project::SdtmProjectApiError::FkSdtmProjectVersionNotFound(7),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "fk_sdtm_project_version_not_found");
    }

    #[tokio::test]
    async fn sdtm_project_duplicate_maps_to_409() {
        let (status, body) = render_sdtm_project(
            apis::sdtm_project::SdtmProjectApiError::DuplicateSdtmProjectDomain {
                version_id: 1,
                domain_code: "AE".into(),
            },
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_sdtm_project_domain");
    }

    #[tokio::test]
    async fn sdtm_project_repository_maps_to_500() {
        let (status, body) = render_sdtm_project(
            apis::sdtm_project::SdtmProjectApiError::Repository("boom".into()),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    async fn render_crf(err: apis::crf::CrfApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    #[tokio::test]
    async fn crf_validation_maps_to_400() {
        let (status, body) = render_crf(apis::crf::CrfApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn crf_empty_search_fragment_maps_to_400() {
        let (status, body) = render_crf(apis::crf::CrfApiError::EmptySearchFragment).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "empty_search_fragment");
    }

    #[tokio::test]
    async fn crf_kind_shape_violation_maps_to_400() {
        let (status, body) = render_crf(apis::crf::CrfApiError::KindShapeViolation {
            kind: apis::crf::CrfItemKind::Selection,
            field: "options".into(),
        })
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "kind_shape_violation");
    }

    #[tokio::test]
    async fn crf_not_found_maps_to_404() {
        let (status, body) = render_crf(apis::crf::CrfApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn crf_project_not_found_maps_to_404() {
        let (status, body) = render_crf(apis::crf::CrfApiError::ProjectNotFound("P1".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "project_not_found");
    }

    #[tokio::test]
    async fn crf_version_not_found_maps_to_404() {
        let (status, body) = render_crf(apis::crf::CrfApiError::CrfVersionNotFound(7)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "crf_version_not_found");
    }

    #[tokio::test]
    async fn crf_form_not_found_maps_to_404() {
        let (status, body) = render_crf(apis::crf::CrfApiError::CrfFormNotFound(7)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "crf_form_not_found");
    }

    #[tokio::test]
    async fn crf_duplicate_version_maps_to_409() {
        let (status, body) = render_crf(apis::crf::CrfApiError::DuplicateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_crf_version");
    }

    #[tokio::test]
    async fn crf_duplicate_form_maps_to_409() {
        let (status, body) = render_crf(apis::crf::CrfApiError::DuplicateCrfForm {
            version_id: 1,
            code: "F1".into(),
        })
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_crf_form");
    }

    #[tokio::test]
    async fn crf_fk_version_not_found_maps_to_400() {
        let (status, body) = render_crf(apis::crf::CrfApiError::FkCrfVersionNotFound(99)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "fk_crf_version_not_found");
    }

    #[tokio::test]
    async fn crf_repository_maps_to_500() {
        let (status, body) = render_crf(apis::crf::CrfApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- MissionApiError mapping (issue variants) -----

    async fn render_mission(err: apis::mission::MissionApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    #[tokio::test]
    async fn mission_issue_not_found_maps_to_404() {
        let (status, body) = render_mission(apis::mission::MissionApiError::IssueNotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "issue_not_found");
    }

    #[tokio::test]
    async fn mission_not_found_for_issue_maps_to_404() {
        let (status, body) =
            render_mission(apis::mission::MissionApiError::MissionNotFoundForIssue(42)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "mission_not_found_for_issue");
    }

    // ---- IngestorError (log-ingest) mapping (new) -----

    use logging_utils::IngestorError;
    use std::path::PathBuf;

    #[test]
    fn log_ingest_dispatch_duplicate_batch_id_is_409() {
        let err = ApiError::LogIngest(IngestorError::DuplicateBatchId("b-1".into()));
        assert_eq!(err.status(), StatusCode::CONFLICT);
        assert_eq!(err.code(), "duplicate_batch_id");
    }

    #[test]
    fn log_ingest_dispatch_channel_full_is_503() {
        let err = ApiError::LogIngest(IngestorError::ChannelFull(1000));
        assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.code(), "channel_full");
    }

    #[test]
    fn log_ingest_dispatch_shut_down_is_503() {
        let err = ApiError::LogIngest(IngestorError::ShutDown);
        assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.code(), "ingestor_shut_down");
    }

    #[test]
    fn log_ingest_dispatch_writer_join_timeout_is_500() {
        let err = ApiError::LogIngest(IngestorError::WriterJoinTimeout {
            deadline: std::time::Duration::from_millis(1),
            message: "test".into(),
        });
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code(), "writer_join_timeout");
    }

    #[test]
    fn log_ingest_dispatch_writer_panic_is_500() {
        let err = ApiError::LogIngest(IngestorError::WriterPanic("disk full".into()));
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code(), "writer_panic");
    }

    #[test]
    fn log_ingest_dispatch_create_dir_is_500() {
        let err = ApiError::LogIngest(IngestorError::CreateDir {
            dir: PathBuf::from("/nope"),
            source: std::io::Error::other("test"),
        });
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code(), "log_dir_create_failed");
    }
}
