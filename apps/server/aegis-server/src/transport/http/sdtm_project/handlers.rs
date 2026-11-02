//! HTTP handlers for the sdtm-project namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from `dto`) into an apis DTO.
//! 2. Calls the corresponding
//!    [`SdtmProjectService`](apis::sdtm_project::SdtmProjectService)
//!    method on `AppState`.
//! 3. Translates the apis response back into a wire DTO.
//!
//! `SdtmProjectApiError` is funnelled through [`ApiError::from`] so
//! each route returns `Result<Json<T>, ApiError>` and the error
//! mapping in `transport::http::error` does the rest.
//!
//! The namespace has **no role gate**: any authenticated user may read
//! and write, matching the `crf` module's policy for project-scoped
//! clinical content. The routes therefore do not advertise a `403`.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto;
use crate::transport::http::error::ApiError;

// ---- SdtmProjectVersion ----

/// `POST /api/sdtm-project/versions` — create a project SDTM version.
#[utoipa::path(
    post, path = "/versions", tag = "sdtm-project",
    operation_id = "sdtm_project_create_version",
    request_body = dto::CreateSdtmProjectVersionRequest,
    responses(
        (status = 201, description = "Version created", body = dto::SdtmProjectVersionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate sdtm project version", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<dto::CreateSdtmProjectVersionRequest>,
) -> Result<(StatusCode, Json<dto::SdtmProjectVersionViewResponse>), ApiError> {
    let view = state
        .sdtm_project
        .create_version(apis::sdtm_project::CreateSdtmProjectVersionRequest {
            project_code: req.project_code,
            version_name: req.version_name,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/sdtm-project/versions` — list a project's versions.
#[utoipa::path(
    get, path = "/versions", tag = "sdtm-project",
    operation_id = "sdtm_project_list_versions_by_project",
    params(
        ("projectCode" = String, Query, description = "Project code the versions belong to"),
    ),
    responses(
        (status = 200, description = "Versions list", body = dto::SdtmProjectVersionListResponse),
        (status = 400, description = "Missing projectCode query parameter", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_versions_by_project(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(query): Query<dto::SdtmProjectVersionListQuery>,
) -> Result<Json<dto::SdtmProjectVersionListResponse>, ApiError> {
    let out = state
        .sdtm_project
        .list_versions_by_project(&query.project_code)
        .await?;
    Ok(Json(dto::SdtmProjectVersionListResponse {
        versions: out.versions.into_iter().map(Into::into).collect(),
    }))
}

/// `GET /api/sdtm-project/versions/{id}` — fetch a version by id.
#[utoipa::path(
    get, path = "/versions/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_get_version_by_id",
    params(
        ("id" = i64, Path, description = "Sdtm project version id"),
    ),
    responses(
        (status = 200, description = "Version found", body = dto::SdtmProjectVersionViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_version_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::SdtmProjectVersionViewResponse>, ApiError> {
    let view = state.sdtm_project.get_version_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `PUT /api/sdtm-project/versions/{id}` — partial update of a version.
#[utoipa::path(
    put, path = "/versions/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_update_version",
    params(
        ("id" = i64, Path, description = "Sdtm project version id"),
    ),
    request_body = dto::UpdateSdtmProjectVersionRequest,
    responses(
        (status = 200, description = "Version updated", body = dto::SdtmProjectVersionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateSdtmProjectVersionRequest>,
) -> Result<Json<dto::SdtmProjectVersionViewResponse>, ApiError> {
    let view = state
        .sdtm_project
        .update_version(apis::sdtm_project::UpdateSdtmProjectVersionRequest {
            id,
            version_name: req.version_name,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/sdtm-project/versions/{id}` — hard delete a version.
#[utoipa::path(
    delete, path = "/versions/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_delete_version",
    params(
        ("id" = i64, Path, description = "Sdtm project version id"),
    ),
    responses(
        (status = 204, description = "Version deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    state.sdtm_project.delete_version(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- SdtmProjectDomain ----

/// `POST /api/sdtm-project/domains` — create a project domain.
#[utoipa::path(
    post, path = "/domains", tag = "sdtm-project",
    operation_id = "sdtm_project_create_domain",
    request_body = dto::CreateSdtmProjectDomainRequest,
    responses(
        (status = 201, description = "Domain created", body = dto::SdtmProjectDomainViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Parent version not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate sdtm project domain", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_domain(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<dto::CreateSdtmProjectDomainRequest>,
) -> Result<(StatusCode, Json<dto::SdtmProjectDomainViewResponse>), ApiError> {
    let view = state
        .sdtm_project
        .create_domain(apis::sdtm_project::CreateSdtmProjectDomainRequest {
            version_id: req.version_id,
            domain_code: req.domain_code,
            key_variables: req.key_variables,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/sdtm-project/domains/{id}` — fetch a domain by id.
#[utoipa::path(
    get, path = "/domains/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_get_domain_by_id",
    params(
        ("id" = i64, Path, description = "Sdtm project domain id"),
    ),
    responses(
        (status = 200, description = "Domain found", body = dto::SdtmProjectDomainViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Domain not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_domain_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::SdtmProjectDomainViewResponse>, ApiError> {
    let view = state.sdtm_project.get_domain_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `GET /api/sdtm-project/versions/{version_id}/domains` — list domains under a version.
#[utoipa::path(
    get, path = "/versions/{version_id}/domains", tag = "sdtm-project",
    operation_id = "sdtm_project_list_domains_by_version",
    params(
        ("version_id" = i64, Path, description = "Sdtm project version id"),
    ),
    responses(
        (status = 200, description = "Domains list", body = dto::SdtmProjectDomainListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_domains_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(version_id): Path<i64>,
) -> Result<Json<dto::SdtmProjectDomainListResponse>, ApiError> {
    let out = state
        .sdtm_project
        .list_domains_by_version(version_id)
        .await?;
    Ok(Json(dto::SdtmProjectDomainListResponse {
        domains: out.domains.into_iter().map(Into::into).collect(),
    }))
}

/// `PUT /api/sdtm-project/domains/{id}` — partial update of a domain.
#[utoipa::path(
    put, path = "/domains/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_update_domain",
    params(
        ("id" = i64, Path, description = "Sdtm project domain id"),
    ),
    request_body = dto::UpdateSdtmProjectDomainRequest,
    responses(
        (status = 200, description = "Domain updated", body = dto::SdtmProjectDomainViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Domain not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_domain(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateSdtmProjectDomainRequest>,
) -> Result<Json<dto::SdtmProjectDomainViewResponse>, ApiError> {
    let view = state
        .sdtm_project
        .update_domain(apis::sdtm_project::UpdateSdtmProjectDomainRequest {
            id,
            domain_code: req.domain_code,
            key_variables: req.key_variables,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/sdtm-project/domains/{id}` — hard delete a domain.
#[utoipa::path(
    delete, path = "/domains/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_delete_domain",
    params(
        ("id" = i64, Path, description = "Sdtm project domain id"),
    ),
    responses(
        (status = 204, description = "Domain deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Domain not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_domain(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    state.sdtm_project.delete_domain(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- SdtmProjectDomainOverride ----

/// `POST /api/sdtm-project/domains/{domain_id}/override` — create the domain's override.
#[utoipa::path(
    post, path = "/domains/{domain_id}/override", tag = "sdtm-project",
    operation_id = "sdtm_project_create_domain_override",
    params(
        ("domain_id" = i64, Path, description = "Sdtm project domain id"),
    ),
    request_body = dto::CreateSdtmProjectDomainOverrideRequest,
    responses(
        (status = 201, description = "Override created", body = dto::SdtmProjectDomainOverrideViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Parent domain not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "The domain already holds an override", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_domain_override(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(domain_id): Path<i64>,
    Json(req): Json<dto::CreateSdtmProjectDomainOverrideRequest>,
) -> Result<(StatusCode, Json<dto::SdtmProjectDomainOverrideViewResponse>), ApiError> {
    let view = state
        .sdtm_project
        .create_domain_override(apis::sdtm_project::CreateSdtmProjectDomainOverrideRequest {
            domain_id,
            category: req.category.into(),
            descriptions: req.descriptions.into_iter().map(Into::into).collect(),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/sdtm-project/domains/{domain_id}/override` — fetch the domain's override.
///
/// The override is optional, so a domain without one is **not** an
/// error: the response is `200` with `{"override": null}`.
#[utoipa::path(
    get, path = "/domains/{domain_id}/override", tag = "sdtm-project",
    operation_id = "sdtm_project_get_domain_override",
    params(
        ("domain_id" = i64, Path, description = "Sdtm project domain id"),
    ),
    responses(
        (status = 200, description = "The domain's override, or null when it has none", body = dto::SdtmProjectDomainOverrideResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_domain_override(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(domain_id): Path<i64>,
) -> Result<Json<dto::SdtmProjectDomainOverrideResponse>, ApiError> {
    let view = state.sdtm_project.get_domain_override(domain_id).await?;
    Ok(Json(dto::SdtmProjectDomainOverrideResponse {
        domain_override: view.map(Into::into),
    }))
}

/// `PUT /api/sdtm-project/domains/{domain_id}/override` — update the domain's override.
#[utoipa::path(
    put, path = "/domains/{domain_id}/override", tag = "sdtm-project",
    operation_id = "sdtm_project_update_domain_override",
    params(
        ("domain_id" = i64, Path, description = "Sdtm project domain id"),
    ),
    request_body = dto::UpdateSdtmProjectDomainOverrideRequest,
    responses(
        (status = 200, description = "Override updated", body = dto::SdtmProjectDomainOverrideViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Override not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_domain_override(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(domain_id): Path<i64>,
    Json(req): Json<dto::UpdateSdtmProjectDomainOverrideRequest>,
) -> Result<Json<dto::SdtmProjectDomainOverrideViewResponse>, ApiError> {
    let view = state
        .sdtm_project
        .update_domain_override(apis::sdtm_project::UpdateSdtmProjectDomainOverrideRequest {
            domain_id,
            category: req.category.map(Into::into),
            descriptions: req
                .descriptions
                .map(|ds| ds.into_iter().map(Into::into).collect()),
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/sdtm-project/domains/{domain_id}/override` — delete the domain's override.
#[utoipa::path(
    delete, path = "/domains/{domain_id}/override", tag = "sdtm-project",
    operation_id = "sdtm_project_delete_domain_override",
    params(
        ("domain_id" = i64, Path, description = "Sdtm project domain id"),
    ),
    responses(
        (status = 204, description = "Override deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Override not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_domain_override(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(domain_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state.sdtm_project.delete_domain_override(domain_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- SdtmProjectVariable ----

/// `POST /api/sdtm-project/variables` — create a project variable.
#[utoipa::path(
    post, path = "/variables", tag = "sdtm-project",
    operation_id = "sdtm_project_create_variable",
    request_body = dto::CreateSdtmProjectVariableRequest,
    responses(
        (status = 201, description = "Variable created", body = dto::SdtmProjectVariableViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Parent domain not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate sdtm project variable", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_variable(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<dto::CreateSdtmProjectVariableRequest>,
) -> Result<(StatusCode, Json<dto::SdtmProjectVariableViewResponse>), ApiError> {
    let view = state
        .sdtm_project
        .create_variable(apis::sdtm_project::CreateSdtmProjectVariableRequest {
            domain_id: req.domain_id,
            variable_code: req.variable_code,
            variable_sequence: req.variable_sequence,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/sdtm-project/variables/{id}` — fetch a variable by id.
#[utoipa::path(
    get, path = "/variables/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_get_variable_by_id",
    params(
        ("id" = i64, Path, description = "Sdtm project variable id"),
    ),
    responses(
        (status = 200, description = "Variable found", body = dto::SdtmProjectVariableViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Variable not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_variable_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::SdtmProjectVariableViewResponse>, ApiError> {
    let view = state.sdtm_project.get_variable_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `GET /api/sdtm-project/domains/{domain_id}/variables` — list variables under a domain.
#[utoipa::path(
    get, path = "/domains/{domain_id}/variables", tag = "sdtm-project",
    operation_id = "sdtm_project_list_variables_by_domain",
    params(
        ("domain_id" = i64, Path, description = "Sdtm project domain id"),
    ),
    responses(
        (status = 200, description = "Variables list", body = dto::SdtmProjectVariableListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_variables_by_domain(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(domain_id): Path<i64>,
) -> Result<Json<dto::SdtmProjectVariableListResponse>, ApiError> {
    let out = state
        .sdtm_project
        .list_variables_by_domain(domain_id)
        .await?;
    Ok(Json(dto::SdtmProjectVariableListResponse {
        variables: out.variables.into_iter().map(Into::into).collect(),
    }))
}

/// `PUT /api/sdtm-project/variables/{id}` — partial update of a variable.
#[utoipa::path(
    put, path = "/variables/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_update_variable",
    params(
        ("id" = i64, Path, description = "Sdtm project variable id"),
    ),
    request_body = dto::UpdateSdtmProjectVariableRequest,
    responses(
        (status = 200, description = "Variable updated", body = dto::SdtmProjectVariableViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Variable not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_variable(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateSdtmProjectVariableRequest>,
) -> Result<Json<dto::SdtmProjectVariableViewResponse>, ApiError> {
    let view = state
        .sdtm_project
        .update_variable(apis::sdtm_project::UpdateSdtmProjectVariableRequest {
            id,
            variable_code: req.variable_code,
            variable_sequence: req.variable_sequence,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/sdtm-project/variables/{id}` — hard delete a variable.
#[utoipa::path(
    delete, path = "/variables/{id}", tag = "sdtm-project",
    operation_id = "sdtm_project_delete_variable",
    params(
        ("id" = i64, Path, description = "Sdtm project variable id"),
    ),
    responses(
        (status = 204, description = "Variable deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Variable not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_variable(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    state.sdtm_project.delete_variable(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- SdtmProjectVariableOverride ----

/// `POST /api/sdtm-project/variables/{variable_id}/override` — create the variable's override.
#[utoipa::path(
    post, path = "/variables/{variable_id}/override", tag = "sdtm-project",
    operation_id = "sdtm_project_create_variable_override",
    params(
        ("variable_id" = i64, Path, description = "Sdtm project variable id"),
    ),
    request_body = dto::CreateSdtmProjectVariableOverrideRequest,
    responses(
        (status = 201, description = "Override created", body = dto::SdtmProjectVariableOverrideViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Parent variable not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "The variable already holds an override", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_variable_override(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(variable_id): Path<i64>,
    Json(req): Json<dto::CreateSdtmProjectVariableOverrideRequest>,
) -> Result<
    (
        StatusCode,
        Json<dto::SdtmProjectVariableOverrideViewResponse>,
    ),
    ApiError,
> {
    let view = state
        .sdtm_project
        .create_variable_override(
            apis::sdtm_project::CreateSdtmProjectVariableOverrideRequest {
                variable_id,
                variable_type: req.variable_type.into(),
                variable_core: req.variable_core.into(),
                variable_role: req.variable_role.map(Into::into),
                descriptions: req.descriptions.into_iter().map(Into::into).collect(),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/sdtm-project/variables/{variable_id}/override` — fetch the variable's override.
///
/// The override is optional, so a variable without one is **not** an
/// error: the response is `200` with `{"override": null}`.
#[utoipa::path(
    get, path = "/variables/{variable_id}/override", tag = "sdtm-project",
    operation_id = "sdtm_project_get_variable_override",
    params(
        ("variable_id" = i64, Path, description = "Sdtm project variable id"),
    ),
    responses(
        (status = 200, description = "The variable's override, or null when it has none", body = dto::SdtmProjectVariableOverrideResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_variable_override(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(variable_id): Path<i64>,
) -> Result<Json<dto::SdtmProjectVariableOverrideResponse>, ApiError> {
    let view = state
        .sdtm_project
        .get_variable_override(variable_id)
        .await?;
    Ok(Json(dto::SdtmProjectVariableOverrideResponse {
        variable_override: view.map(Into::into),
    }))
}

/// `PUT /api/sdtm-project/variables/{variable_id}/override` — update the variable's override.
#[utoipa::path(
    put, path = "/variables/{variable_id}/override", tag = "sdtm-project",
    operation_id = "sdtm_project_update_variable_override",
    params(
        ("variable_id" = i64, Path, description = "Sdtm project variable id"),
    ),
    request_body = dto::UpdateSdtmProjectVariableOverrideRequest,
    responses(
        (status = 200, description = "Override updated", body = dto::SdtmProjectVariableOverrideViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Override not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_variable_override(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(variable_id): Path<i64>,
    Json(req): Json<dto::UpdateSdtmProjectVariableOverrideRequest>,
) -> Result<Json<dto::SdtmProjectVariableOverrideViewResponse>, ApiError> {
    let view = state
        .sdtm_project
        .update_variable_override(
            apis::sdtm_project::UpdateSdtmProjectVariableOverrideRequest {
                variable_id,
                variable_type: req.variable_type.map(Into::into),
                variable_core: req.variable_core.map(Into::into),
                variable_role: req.variable_role.map(|inner| inner.map(Into::into)),
                descriptions: req
                    .descriptions
                    .map(|ds| ds.into_iter().map(Into::into).collect()),
            },
        )
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/sdtm-project/variables/{variable_id}/override` — delete the variable's override.
#[utoipa::path(
    delete, path = "/variables/{variable_id}/override", tag = "sdtm-project",
    operation_id = "sdtm_project_delete_variable_override",
    params(
        ("variable_id" = i64, Path, description = "Sdtm project variable id"),
    ),
    responses(
        (status = 204, description = "Override deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Override not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_variable_override(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(variable_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state
        .sdtm_project
        .delete_variable_override(variable_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    //! Handler tests for `/api/sdtm-project`.
    //!
    //! `MockSdtmProjectService` answers every service method
    //! successfully and `MockAuth` accepts any bearer token, so the
    //! tests exercise routing and extraction rather than behaviour.
    //!
    //! `all_routes_dispatch_to_their_handler` drives **every** route
    //! with a concrete path and asserts the status the handler
    //! returns. That is the test that catches a route whose path
    //! label does not match the extractor's field name — a mismatch
    //! that leaves the OpenAPI document (built from the
    //! `#[utoipa::path]` annotations) looking perfectly correct.

    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::state::test_support::{
        NullCrfService, NullDomainModelService, NullMissionService, NullTerminologyService,
    };

    use apis::auth::{
        AuthApiError, AuthClaims as ApisAuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };

    use apis::sdtm_project::{
        CreateSdtmProjectDomainOverrideRequest, CreateSdtmProjectDomainRequest,
        CreateSdtmProjectVariableOverrideRequest, CreateSdtmProjectVariableRequest,
        CreateSdtmProjectVersionRequest, SdtmProjectApiError, SdtmProjectService,
        UpdateSdtmProjectDomainOverrideRequest, UpdateSdtmProjectDomainRequest,
        UpdateSdtmProjectVariableOverrideRequest, UpdateSdtmProjectVariableRequest,
        UpdateSdtmProjectVersionRequest,
    };

    // ---- mock auth ----------------------------------------------------

    #[derive(Clone)]
    pub struct MockAuth;

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self,
            _req: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> {
            unimplemented!()
        }
        async fn login_with_domain_user_info(
            &self,
            _req: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> {
            unimplemented!()
        }
        async fn verify(&self, _req: VerifyRequest) -> Result<ApisAuthClaims, AuthApiError> {
            Ok(ApisAuthClaims {
                code: "u1".into(),
                role: apis::user::Role::Admin,
                token_version: 0,
            })
        }
        async fn refresh(&self, _req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
            unimplemented!()
        }
        async fn find_user_credential_by_code(
            &self,
            _code: &str,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn create_user_credential(
            &self,
            _req: CreateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn update_user_credential(
            &self,
            _req: UpdateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn remove_user_credential(
            &self,
            _code: &str,
        ) -> Result<RemoveUserCredentialResponse, AuthApiError> {
            unimplemented!()
        }
        async fn logout(&self, _req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
            unimplemented!()
        }
        async fn register_user(
            &self,
            _req: RegisterUserRequest,
        ) -> Result<RegisterUserResponse, AuthApiError> {
            unimplemented!("not exercised by this handler")
        }
    }

    // ---- mock sdtm-project service ------------------------------------
    //
    // Every method answers successfully, so a failing route can only
    // be a routing or extraction failure.

    #[derive(Clone, Default)]
    pub struct MockSdtmProjectService {
        /// The last domain the handler asked to create.
        pub created_domain: Arc<std::sync::Mutex<Option<CreateSdtmProjectDomainRequest>>>,
        /// When true, the override getters answer `None` — the state a
        /// domain or variable that has never been overridden is in.
        pub override_absent: bool,
    }

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    fn version_view() -> apis::sdtm_project::SdtmProjectVersionView {
        apis::sdtm_project::SdtmProjectVersionView {
            id: 1,
            project_code: "P1".into(),
            version_name: "2024-09-27".into(),
            created_at: ts(),
            updated_at: ts(),
        }
    }

    fn domain_view() -> apis::sdtm_project::SdtmProjectDomainView {
        apis::sdtm_project::SdtmProjectDomainView {
            id: 2,
            version_id: 1,
            domain_code: "AE".into(),
            key_variables: vec!["AETERM".into()],
            domain_override: None,
            created_at: ts(),
            updated_at: ts(),
        }
    }

    fn domain_override_view() -> apis::sdtm_project::SdtmProjectDomainOverrideView {
        apis::sdtm_project::SdtmProjectDomainOverrideView {
            id: 3,
            domain_id: 2,
            category: apis::sdtm_project::DomainCategory::Events,
            descriptions: Vec::new(),
            created_at: ts(),
            updated_at: ts(),
        }
    }

    fn variable_view() -> apis::sdtm_project::SdtmProjectVariableView {
        apis::sdtm_project::SdtmProjectVariableView {
            id: 4,
            domain_id: 2,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
            variable_override: None,
            created_at: ts(),
            updated_at: ts(),
        }
    }

    fn variable_override_view() -> apis::sdtm_project::SdtmProjectVariableOverrideView {
        apis::sdtm_project::SdtmProjectVariableOverrideView {
            id: 5,
            variable_id: 4,
            variable_type: apis::sdtm_project::SdtmVariableType::Character,
            variable_core: apis::sdtm_project::SdtmVariableCore::Req,
            variable_role: Some(apis::sdtm_project::SdtmRole::Topic),
            descriptions: Vec::new(),
            created_at: ts(),
            updated_at: ts(),
        }
    }

    #[async_trait]
    impl SdtmProjectService for MockSdtmProjectService {
        async fn create_version(
            &self,
            _req: CreateSdtmProjectVersionRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectVersionView, SdtmProjectApiError> {
            Ok(version_view())
        }
        async fn get_version_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::sdtm_project::SdtmProjectVersionView, SdtmProjectApiError> {
            Ok(version_view())
        }
        async fn list_versions_by_project(
            &self,
            _project_code: &str,
        ) -> Result<apis::sdtm_project::SdtmProjectVersionList, SdtmProjectApiError> {
            Ok(apis::sdtm_project::SdtmProjectVersionList {
                versions: vec![version_view()],
            })
        }
        async fn update_version(
            &self,
            _req: UpdateSdtmProjectVersionRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectVersionView, SdtmProjectApiError> {
            Ok(version_view())
        }
        async fn delete_version(&self, _id: i64) -> Result<(), SdtmProjectApiError> {
            Ok(())
        }

        async fn create_domain(
            &self,
            req: CreateSdtmProjectDomainRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectDomainView, SdtmProjectApiError> {
            *self.created_domain.lock().unwrap() = Some(req);
            Ok(domain_view())
        }
        async fn get_domain_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::sdtm_project::SdtmProjectDomainView, SdtmProjectApiError> {
            Ok(domain_view())
        }
        async fn list_domains_by_version(
            &self,
            _version_id: i64,
        ) -> Result<apis::sdtm_project::SdtmProjectDomainList, SdtmProjectApiError> {
            Ok(apis::sdtm_project::SdtmProjectDomainList {
                domains: vec![domain_view()],
            })
        }
        async fn update_domain(
            &self,
            _req: UpdateSdtmProjectDomainRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectDomainView, SdtmProjectApiError> {
            Ok(domain_view())
        }
        async fn delete_domain(&self, _id: i64) -> Result<(), SdtmProjectApiError> {
            Ok(())
        }

        async fn create_domain_override(
            &self,
            _req: CreateSdtmProjectDomainOverrideRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectDomainOverrideView, SdtmProjectApiError>
        {
            Ok(domain_override_view())
        }
        async fn get_domain_override(
            &self,
            _domain_id: i64,
        ) -> Result<Option<apis::sdtm_project::SdtmProjectDomainOverrideView>, SdtmProjectApiError>
        {
            Ok(if self.override_absent {
                None
            } else {
                Some(domain_override_view())
            })
        }
        async fn update_domain_override(
            &self,
            _req: UpdateSdtmProjectDomainOverrideRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectDomainOverrideView, SdtmProjectApiError>
        {
            Ok(domain_override_view())
        }
        async fn delete_domain_override(&self, _domain_id: i64) -> Result<(), SdtmProjectApiError> {
            Ok(())
        }

        async fn create_variable(
            &self,
            _req: CreateSdtmProjectVariableRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectVariableView, SdtmProjectApiError> {
            Ok(variable_view())
        }
        async fn get_variable_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::sdtm_project::SdtmProjectVariableView, SdtmProjectApiError> {
            Ok(variable_view())
        }
        async fn list_variables_by_domain(
            &self,
            _domain_id: i64,
        ) -> Result<apis::sdtm_project::SdtmProjectVariableList, SdtmProjectApiError> {
            Ok(apis::sdtm_project::SdtmProjectVariableList {
                variables: vec![variable_view()],
            })
        }
        async fn update_variable(
            &self,
            _req: UpdateSdtmProjectVariableRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectVariableView, SdtmProjectApiError> {
            Ok(variable_view())
        }
        async fn delete_variable(&self, _id: i64) -> Result<(), SdtmProjectApiError> {
            Ok(())
        }

        async fn create_variable_override(
            &self,
            _req: CreateSdtmProjectVariableOverrideRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectVariableOverrideView, SdtmProjectApiError>
        {
            Ok(variable_override_view())
        }
        async fn get_variable_override(
            &self,
            _variable_id: i64,
        ) -> Result<Option<apis::sdtm_project::SdtmProjectVariableOverrideView>, SdtmProjectApiError>
        {
            Ok(if self.override_absent {
                None
            } else {
                Some(variable_override_view())
            })
        }
        async fn update_variable_override(
            &self,
            _req: UpdateSdtmProjectVariableOverrideRequest,
        ) -> Result<apis::sdtm_project::SdtmProjectVariableOverrideView, SdtmProjectApiError>
        {
            Ok(variable_override_view())
        }
        async fn delete_variable_override(
            &self,
            _variable_id: i64,
        ) -> Result<(), SdtmProjectApiError> {
            Ok(())
        }
    }

    // ---- harness ------------------------------------------------------

    pub fn test_state(sdtm_project: MockSdtmProjectService) -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(crate::state::test_support::NullUserService)
                as Arc<dyn apis::user::UserService>,
            project: Arc::new(crate::state::test_support::NullProjectService)
                as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(NullMissionService) as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(NullCrfService) as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(sdtm_project) as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

    /// The **real** top-level router, so the `/api/sdtm-project` mount
    /// and every path parameter are exercised exactly as a client
    /// would meet them.
    pub fn app(state: AppState) -> Router {
        crate::transport::http::router::router(state)
    }

    pub async fn read_json(response: axum::response::Response) -> (AxStatus, serde_json::Value) {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    pub fn build_request(
        method: &str,
        uri: &str,
        body: Option<String>,
        auth: Option<&str>,
    ) -> Request<Body> {
        let mut b = Request::builder().method(method).uri(uri);
        if let Some(token) = auth {
            b = b.header("authorization", token);
        }
        if body.is_some() {
            b = b.header("content-type", "application/json");
        }
        b.body(body.map(Body::from).unwrap_or(Body::empty()))
            .unwrap()
    }

    // ---- the dispatch test --------------------------------------------

    /// Every route must dispatch to its handler.
    ///
    /// axum matches a path segment's label to the extractor's field
    /// name, so a route declared as `{domain_id}` cannot be extracted
    /// with a struct whose only field is `id` — the request is
    /// rejected with `400 Invalid URL: missing field 'id'` before the
    /// handler ever runs, while the OpenAPI document (built from the
    /// `#[utoipa::path]` annotations) still looks correct. This test
    /// is the guard against that class of defect.
    #[tokio::test]
    async fn all_routes_dispatch_to_their_handler() {
        // (method, uri, request body, expected status)
        let cases: Vec<(&str, &str, Option<&str>, AxStatus)> = vec![
            (
                "POST",
                "/api/sdtm-project/versions",
                Some(r#"{"projectCode":"P1","versionName":"2024-09-27"}"#),
                AxStatus::CREATED,
            ),
            (
                "GET",
                "/api/sdtm-project/versions?projectCode=P1",
                None,
                AxStatus::OK,
            ),
            ("GET", "/api/sdtm-project/versions/1", None, AxStatus::OK),
            (
                "PUT",
                "/api/sdtm-project/versions/1",
                Some(r#"{"versionName":"2024-09-27"}"#),
                AxStatus::OK,
            ),
            (
                "DELETE",
                "/api/sdtm-project/versions/1",
                None,
                AxStatus::NO_CONTENT,
            ),
            (
                "POST",
                "/api/sdtm-project/domains",
                Some(r#"{"versionId":1,"domainCode":"AE","keyVariables":["AETERM"]}"#),
                AxStatus::CREATED,
            ),
            ("GET", "/api/sdtm-project/domains/2", None, AxStatus::OK),
            (
                "GET",
                "/api/sdtm-project/versions/1/domains",
                None,
                AxStatus::OK,
            ),
            (
                "PUT",
                "/api/sdtm-project/domains/2",
                Some(r#"{"domainCode":"AE"}"#),
                AxStatus::OK,
            ),
            (
                "DELETE",
                "/api/sdtm-project/domains/2",
                None,
                AxStatus::NO_CONTENT,
            ),
            (
                "POST",
                "/api/sdtm-project/domains/2/override",
                Some(r#"{"category":"Events","descriptions":[]}"#),
                AxStatus::CREATED,
            ),
            (
                "GET",
                "/api/sdtm-project/domains/2/override",
                None,
                AxStatus::OK,
            ),
            (
                "PUT",
                "/api/sdtm-project/domains/2/override",
                Some(r#"{"category":"Findings"}"#),
                AxStatus::OK,
            ),
            (
                "DELETE",
                "/api/sdtm-project/domains/2/override",
                None,
                AxStatus::NO_CONTENT,
            ),
            (
                "POST",
                "/api/sdtm-project/variables",
                Some(r#"{"domainId":2,"variableCode":"AETERM","variableSequence":11}"#),
                AxStatus::CREATED,
            ),
            ("GET", "/api/sdtm-project/variables/4", None, AxStatus::OK),
            (
                "GET",
                "/api/sdtm-project/domains/2/variables",
                None,
                AxStatus::OK,
            ),
            (
                "PUT",
                "/api/sdtm-project/variables/4",
                Some(r#"{"variableCode":"AETERM"}"#),
                AxStatus::OK,
            ),
            (
                "DELETE",
                "/api/sdtm-project/variables/4",
                None,
                AxStatus::NO_CONTENT,
            ),
            (
                "POST",
                "/api/sdtm-project/variables/4/override",
                Some(r#"{"variableType":"Character","variableCore":"Req","descriptions":[]}"#),
                AxStatus::CREATED,
            ),
            (
                "GET",
                "/api/sdtm-project/variables/4/override",
                None,
                AxStatus::OK,
            ),
            (
                "PUT",
                "/api/sdtm-project/variables/4/override",
                Some(r#"{"variableType":"Character"}"#),
                AxStatus::OK,
            ),
            (
                "DELETE",
                "/api/sdtm-project/variables/4/override",
                None,
                AxStatus::NO_CONTENT,
            ),
        ];

        for (method, uri, body, expected) in cases {
            let state = test_state(MockSdtmProjectService::default());
            let response = app(state)
                .oneshot(build_request(
                    method,
                    uri,
                    body.map(str::to_string),
                    Some("Bearer good"),
                ))
                .await
                .unwrap();
            let status = response.status();
            assert_eq!(
                status, expected,
                "{method} {uri} returned {status}, expected {expected}"
            );
        }
    }

    // ---- focused behaviour --------------------------------------------

    /// `GET /api/sdtm-project/versions` needs a `projectCode` query
    /// parameter. Without it the request is rejected with 400.
    #[tokio::test]
    async fn list_versions_without_project_code_is_rejected() {
        let state = test_state(MockSdtmProjectService::default());
        let response = app(state)
            .oneshot(build_request(
                "GET",
                "/api/sdtm-project/versions",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::BAD_REQUEST);
    }

    /// `POST /api/sdtm-project/domains` returns 201 and the created
    /// domain.
    #[tokio::test]
    async fn create_domain_returns_created() {
        let service = MockSdtmProjectService::default();
        let state = test_state(service.clone());
        let response = app(state)
            .oneshot(build_request(
                "POST",
                "/api/sdtm-project/domains",
                Some(
                    r#"{"versionId":1,"domainCode":"AE","keyVariables":["AESEV","AETERM"]}"#.into(),
                ),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::CREATED);
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CREATED);
        assert_eq!(body["domainCode"], serde_json::json!("AE"));
        assert_eq!(body["versionId"], serde_json::json!(1));
        // The nested override is absent: the key is `override`, and
        // the field is `None` here.
        assert!(body["override"].is_null());

        // The request reached the service with the caller's order.
        let recorded = service.created_domain.lock().unwrap().clone().unwrap();
        assert_eq!(
            recorded.key_variables,
            vec!["AESEV".to_string(), "AETERM".to_string()]
        );
    }

    /// The namespace has no role gate, but it does require a token.
    #[tokio::test]
    async fn create_domain_without_authorization_returns_401() {
        let state = test_state(MockSdtmProjectService::default());
        let response = app(state)
            .oneshot(build_request(
                "POST",
                "/api/sdtm-project/domains",
                Some(r#"{"versionId":1,"domainCode":"AE","keyVariables":[]}"#.into()),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }

    /// An override is optional. A domain that has never been
    /// overridden is a normal state, so reading its override is `200`
    /// with `{"override": null}` — not `404`, and not a bare `null`
    /// body.
    #[tokio::test]
    async fn get_domain_override_without_an_override_returns_a_null_override() {
        let service = MockSdtmProjectService {
            override_absent: true,
            ..Default::default()
        };
        let state = test_state(service);
        let response = app(state)
            .oneshot(build_request(
                "GET",
                "/api/sdtm-project/domains/2/override",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert!(
            body.get("override").is_some_and(serde_json::Value::is_null),
            "expected {{\"override\": null}}, got {body}"
        );
    }

    /// Same for a variable: no override is `200` with
    /// `{"override": null}`.
    #[tokio::test]
    async fn get_variable_override_without_an_override_returns_a_null_override() {
        let service = MockSdtmProjectService {
            override_absent: true,
            ..Default::default()
        };
        let state = test_state(service);
        let response = app(state)
            .oneshot(build_request(
                "GET",
                "/api/sdtm-project/variables/4/override",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert!(
            body.get("override").is_some_and(serde_json::Value::is_null),
            "expected {{\"override\": null}}, got {body}"
        );
    }

    /// When the domain *does* have an override, it is carried under
    /// the same `override` key.
    #[tokio::test]
    async fn get_domain_override_returns_the_override_under_the_override_key() {
        let state = test_state(MockSdtmProjectService::default());
        let response = app(state)
            .oneshot(build_request(
                "GET",
                "/api/sdtm-project/domains/2/override",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let ov = &body["override"];
        assert!(ov.is_object(), "expected an override object, got {body}");
        assert_eq!(ov["domainId"], serde_json::json!(2));
        assert_eq!(ov["category"], serde_json::json!("Events"));
    }

    /// The variable-side twin of
    /// `get_domain_override_returns_the_override_under_the_override_key`.
    #[tokio::test]
    async fn get_variable_override_returns_the_override_under_the_override_key() {
        let state = test_state(MockSdtmProjectService::default());
        let response = app(state)
            .oneshot(build_request(
                "GET",
                "/api/sdtm-project/variables/4/override",
                None,
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        let ov = &body["override"];
        assert!(ov.is_object(), "expected an override object, got {body}");
        assert_eq!(ov["variableId"], serde_json::json!(4));
        assert_eq!(ov["variableType"], serde_json::json!("Character"));
    }
}
