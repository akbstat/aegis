//! OpenAPI document builder for the HTTP transport.
//!
//! `#[derive(ToSchema)]` on the wire DTOs plus `#[utoipa::path(...)]`
//! on the handlers (Task 8) supplies the per-type & per-route schema
//! data. This module stitches them together into a single
//! `utoipa::openapi::OpenApi` document so `router()` can serve
//! `/swagger-ui/` and `/api-docs/openapi.json`.
//!
//! Per-route paths are NOT listed here. The
//! `OpenApiRouter::with_openapi(...)` call in `transport::http::router`
//! wires the handlers via `routes!`, and `utoipa-axum` auto-collects
//! each handler's `#[utoipa::path]` into the document with the
//! `nest("/api/auth", ...)` prefix applied. Listing the handlers here
//! as well would double-register them under the un-prefixed relative
//! path (`/login`, `/login-domain`, `/refresh`, `/logout`).
//!
//! The `BearerAuth` security scheme is registered via a [`Modify`]
//! impl (the same pattern the utoipa-axum README uses). Routes that
//! reference it via `security(("BearerAuth" = []))` are then marked
//! as requiring an `Authorization: Bearer <token>` header in the
//! generated document; swagger-ui renders the lock icon on them.

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::transport::http::dto;
use crate::transport::http::error::ErrorBody;

/// OpenAPI document for the aegis-server HTTP transport.
///
/// `paths(...)` is intentionally omitted: the `OpenApiRouter` in
/// `transport::http::router` already registers every handler via
/// `routes!`, and the `nest("/api/auth", ...)` prefix is applied
/// there. The schema registry below still has to be explicit
/// because `utoipa-axum` only collects paths, not schemas. The
/// `SecurityAddon` modifier registers the `BearerAuth` security
/// scheme that `refresh` and `logout` reference via
/// `security(("BearerAuth" = []))`.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "aegis-server API",
        version = "0.1.0",
        description = "HTTP transport for the aegis auth + user + project + terminology + domain-model + crf + mission + structures services."
    ),
    modifiers(&SecurityAddon),
    components(schemas(
        dto::LoginRequest,
        dto::LoginDomainRequest,
        dto::RefreshRequest,
        dto::LogoutRequest,
        dto::TokenPairResponse,
        dto::AccessTokenResponse,
        dto::LogoutResponse,
        dto::AuthClaimsResponse,
        dto::Role,
        dto::CreateUserRequest,
        dto::UpdateUserRequest,
        dto::PathCode,
        dto::UserViewResponse,
        dto::UserListResponse,
        dto::UpdateUserCredentialRequest,
        dto::UserCredentialViewResponse,
        dto::RegisterUserRequest,
        dto::RegisterUserResponse,
        dto::TagDataRequest,
        dto::TagViewResponse,
        dto::ModelVersionRequest,
        dto::ModelVersionResponse,
        dto::CreateProjectRequest,
        dto::UpdateProjectRequest,
        dto::ProjectMemberDataRequest,
        dto::ProjectViewResponse,
        dto::ProjectMemberViewResponse,
        dto::UserSummaryViewResponse,
        dto::ProjectListResponse,
        dto::TerminologyKind,
        dto::PathId,
        dto::TerminologyVersionViewResponse,
        dto::TerminologyVersionListResponse,
        dto::CodeListViewResponse,
        dto::PagedCodeListListResponse,
        dto::CodeItemViewResponse,
        dto::PagedCodeItemListResponse,
        dto::CreateTerminologyVersionRequest,
        dto::UpdateTerminologyVersionRequest,
        dto::CreateCodeListRequest,
        dto::UpdateCodeListRequest,
        dto::CreateCodeItemRequest,
        dto::UpdateCodeItemRequest,
        dto::CodeListListQuery,
        dto::CodeItemListQuery,
        dto::CodeItemByVersionAndCodeQuery,
        dto::DomainCategory,
        dto::SdtmVariableType,
        dto::SdtmVariableCore,
        dto::SdtmRole,
        dto::SdtmDomainDescription,
        dto::SdtmDomainDescriptionDetail,
        dto::SdtmVariableDescription,
        dto::SdtmVariableDescriptionDetail,
        dto::SdtmVariableControlled,
        dto::CreateSdtmVersionRequest,
        dto::UpdateSdtmVersionRequest,
        dto::CreateSdtmDomainRequest,
        dto::UpdateSdtmDomainRequest,
        dto::CreateSdtmVariableRequest,
        dto::UpdateSdtmVariableRequest,
        dto::SdtmVersionViewResponse,
        dto::SdtmVersionListResponse,
        dto::SdtmDomainViewResponse,
        dto::SdtmDomainListResponse,
        dto::SdtmVariableViewResponse,
        dto::SdtmVariableListResponse,
        // -- sdtm-project --
        dto::SdtmProjectDomainCategory,
        dto::SdtmProjectVariableType,
        dto::SdtmProjectVariableCore,
        dto::SdtmProjectRole,
        dto::SdtmProjectDomainDescription,
        dto::SdtmProjectDomainDescriptionDetail,
        dto::SdtmProjectVariableDescription,
        dto::SdtmProjectVariableDescriptionDetail,
        dto::StructureLevelResponse,
        dto::MigrationViewResponse,
        dto::NodeViewResponse,
        dto::StructureViewResponse,
        dto::StructureSummaryResponse,
        dto::StructureMetaResponse,
        dto::StructureTreeResponse,
        dto::OwnNodeResponse,
        dto::OwnNodeListResponse,
        dto::StructureListResponse,
        dto::CreateStructureRequest,
        dto::RenameStructureRequest,
        dto::AddNodeRequest,
        dto::UpdateNodeRequest,
        dto::ListStructuresQuery,
        dto::CreateSdtmProjectVersionRequest,
        dto::UpdateSdtmProjectVersionRequest,
        dto::CreateSdtmProjectDomainRequest,
        dto::UpdateSdtmProjectDomainRequest,
        dto::CreateSdtmProjectDomainOverrideRequest,
        dto::UpdateSdtmProjectDomainOverrideRequest,
        dto::CreateSdtmProjectVariableRequest,
        dto::UpdateSdtmProjectVariableRequest,
        dto::CreateSdtmProjectVariableOverrideRequest,
        dto::UpdateSdtmProjectVariableOverrideRequest,
        dto::SdtmProjectVersionListQuery,
        dto::SdtmProjectVersionViewResponse,
        dto::SdtmProjectVersionListResponse,
        dto::SdtmProjectDomainViewResponse,
        dto::SdtmProjectDomainListResponse,
        dto::SdtmProjectDomainOverrideViewResponse,
        dto::SdtmProjectDomainOverrideResponse,
        dto::SdtmProjectVariableViewResponse,
        dto::SdtmProjectVariableListResponse,
        dto::SdtmProjectVariableOverrideViewResponse,
        dto::SdtmProjectVariableOverrideResponse,
        dto::CrfItemKind,
        dto::AnnotationOwner,
        dto::CrfPathId,
        dto::CrfFragmentQuery,
        dto::ProjectPathCode,
        dto::CrfVersionViewResponse,
        dto::CrfVersionListResponse,
        dto::CrfFormViewResponse,
        dto::CrfFormListResponse,
        dto::CrfItemViewResponse,
        dto::CrfItemListResponse,
        dto::CrfOptionViewResponse,
        dto::CrfOptionListResponse,
        dto::CrfUnitViewResponse,
        dto::CrfUnitListResponse,
        dto::DomainAnnotationViewResponse,
        dto::DomainAnnotationListResponse,
        dto::AnnotationViewResponse,
        dto::AnnotationListResponse,
        dto::CreateCrfVersionRequest,
        dto::UpdateCrfVersionRequest,
        dto::CreateCrfFormRequest,
        dto::UpdateCrfFormRequest,
        dto::BulkCreateCrfFormRequest,
        dto::BulkCreateCrfItemInput,
        dto::BulkCreateCrfFormResponse,
        dto::CreateCrfItemRequest,
        dto::UpdateCrfItemRequest,
        dto::CreateCrfOptionRequest,
        dto::UpdateCrfOptionRequest,
        dto::CreateCrfUnitRequest,
        dto::UpdateCrfUnitRequest,
        dto::CreateDomainAnnotationRequest,
        dto::UpdateDomainAnnotationRequest,
        dto::CreateAnnotationRequest,
        dto::UpdateAnnotationRequest,
        dto::MissionKind,
        dto::MissionRole,
        dto::AssigneeDataRequest,
        dto::CreateMissionRequest,
        dto::AssigneeViewResponse,
        dto::MissionViewResponse,
        dto::MissionListResponse,
        dto::MissionByProjectQuery,
        dto::LogIngestRequest,
        dto::LogIngestResponse,
        ErrorBody,
    )),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "system", description = "Operational endpoints"),
        (name = "user", description = "User CRUD endpoints"),
        (name = "user-credential", description = "User credential self-service endpoints"),
        (name = "project", description = "Project lifecycle endpoints"),
        (name = "terminology", description = "Terminology version / codelist / codeitem endpoints"),
        (name = "domain-model", description = "SDTM domain model version / domain / variable endpoints"),
        (name = "crf", description = "Case Report Form version / form / item / option / unit / annotation endpoints"),
        (name = "mission", description = "Mission lifecycle endpoints (leader-only writes)"),
        (name = "log-ingest", description = "Accepts client log submissions and routes them through the LogIngestor."),
    ),
)]
pub struct ApiDoc;

/// `Modify` impl that registers the `BearerAuth` HTTP security
/// scheme in the OpenAPI document's `components.securitySchemes`.
/// Refresh and logout reference it via
/// `security(("BearerAuth" = []))` from their `#[utoipa::path]`
/// annotations.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "BearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

/// Build the OpenAPI document used by both the swagger-ui and the
/// `/api-docs/openapi.json` endpoint.
pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::openapi::RefOr;

    #[test]
    fn openapi_reports_expected_title_and_version() {
        let doc = openapi();
        assert_eq!(doc.info.title, "aegis-server API");
        assert_eq!(doc.info.version, "0.1.0");
    }

    #[test]
    fn openapi_registers_error_body_schema() {
        let doc = openapi();
        let schemas = &doc
            .components
            .as_ref()
            .expect("components should be present")
            .schemas;
        assert!(
            schemas.contains_key("ErrorBody"),
            "ErrorBody schema missing: {:?}",
            schemas.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn openapi_registers_wire_dto_schemas() {
        let doc = openapi();
        let schemas = &doc.components.as_ref().unwrap().schemas;
        for name in [
            "LoginRequest",
            "LoginDomainRequest",
            "RefreshRequest",
            "LogoutRequest",
            "TokenPairResponse",
            "AccessTokenResponse",
            "LogoutResponse",
            "AuthClaimsResponse",
            "Role",
            "CreateUserRequest",
            "UpdateUserRequest",
            "PathCode",
            "UserViewResponse",
            "UserListResponse",
            "UpdateUserCredentialRequest",
            "UserCredentialViewResponse",
            "TagDataRequest",
            "TagViewResponse",
            "ModelVersionRequest",
            "ModelVersionResponse",
            "CreateProjectRequest",
            "UpdateProjectRequest",
            "ProjectMemberDataRequest",
            "ProjectViewResponse",
            "ProjectMemberViewResponse",
            "UserSummaryViewResponse",
            "ProjectListResponse",
            "TerminologyKind",
            "PathId",
            "TerminologyVersionViewResponse",
            "TerminologyVersionListResponse",
            "CodeListViewResponse",
            "PagedCodeListListResponse",
            "CodeItemViewResponse",
            "PagedCodeItemListResponse",
            "CreateTerminologyVersionRequest",
            "UpdateTerminologyVersionRequest",
            "CreateCodeListRequest",
            "UpdateCodeListRequest",
            "CreateCodeItemRequest",
            "UpdateCodeItemRequest",
            "CodeListListQuery",
            "CodeItemListQuery",
            "CodeItemByVersionAndCodeQuery",
            "DomainCategory",
            "SdtmVariableType",
            "SdtmVariableCore",
            "SdtmRole",
            "SdtmDomainDescription",
            "SdtmDomainDescriptionDetail",
            "SdtmVariableDescription",
            "SdtmVariableDescriptionDetail",
            "SdtmVariableControlled",
            "CreateSdtmVersionRequest",
            "UpdateSdtmVersionRequest",
            "CreateSdtmDomainRequest",
            "UpdateSdtmDomainRequest",
            "CreateSdtmVariableRequest",
            "UpdateSdtmVariableRequest",
            "SdtmVersionViewResponse",
            "SdtmVersionListResponse",
            "SdtmDomainViewResponse",
            "SdtmDomainListResponse",
            "SdtmVariableViewResponse",
            "SdtmVariableListResponse",
            "CrfItemKind",
            "AnnotationOwner",
            "CrfPathId",
            "CrfFragmentQuery",
            "ProjectPathCode",
            "CrfVersionViewResponse",
            "CrfVersionListResponse",
            "CrfFormViewResponse",
            "CrfFormListResponse",
            "CrfItemViewResponse",
            "CrfItemListResponse",
            "CrfOptionViewResponse",
            "CrfOptionListResponse",
            "CrfUnitViewResponse",
            "CrfUnitListResponse",
            "DomainAnnotationViewResponse",
            "DomainAnnotationListResponse",
            "AnnotationViewResponse",
            "AnnotationListResponse",
            "CreateCrfVersionRequest",
            "UpdateCrfVersionRequest",
            "CreateCrfFormRequest",
            "UpdateCrfFormRequest",
            "BulkCreateCrfFormRequest",
            "BulkCreateCrfItemInput",
            "BulkCreateCrfFormResponse",
            "CreateCrfItemRequest",
            "UpdateCrfItemRequest",
            "CreateCrfOptionRequest",
            "UpdateCrfOptionRequest",
            "CreateCrfUnitRequest",
            "UpdateCrfUnitRequest",
            "CreateDomainAnnotationRequest",
            "UpdateDomainAnnotationRequest",
            "CreateAnnotationRequest",
            "UpdateAnnotationRequest",
            "MissionKind",
            "MissionRole",
            "AssigneeDataRequest",
            "CreateMissionRequest",
            "AssigneeViewResponse",
            "MissionViewResponse",
            "MissionListResponse",
            "MissionByProjectQuery",
        ] {
            let entry: &RefOr<_> = schemas
                .get(name)
                .unwrap_or_else(|| panic!("missing schema for {name}"));
            let _ = entry; // schema presence is the assertion
        }
    }
}
