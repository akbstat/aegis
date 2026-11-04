//! HTTP handlers for the structures namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from `dto`) into an apis DTO.
//! 2. Calls the corresponding
//!    [`StructureService`](apis::structures::StructureService) method on
//!    `AppState`.
//! 3. Translates the apis response back into a wire DTO.
//!
//! `StructureApiError` is funnelled through [`ApiError::from`] so
//! each route returns `Result<Json<T>, ApiError>` and the error
//! mapping in `transport::http::error` does the rest.
//!
//! **The auth gate depends on the target, not on the route.** A
//! template structure is global and affects every project, so writing
//! one requires admin/root; a project-level structure is open to any
//! authenticated user. The handler resolves the target's level before
//! the write, so every write route advertises a `403` and no read
//! route does.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use apis::structures::StructureLevel;

use crate::state::AppState;
use crate::transport::http::auth::middleware::{AuthClaims, require_admin_or_root};
use crate::transport::http::dto;
use crate::transport::http::error::ApiError;

/// Reject a write against a template structure unless the caller is an
/// admin or root.
///
/// The level comes from `get_structure_meta`, which does **not** build
/// the effective tree — building the whole merged tree to read one
/// enum would be wasteful on every write.
async fn require_template_write_permission(
    state: &AppState,
    claims: &AuthClaims,
    structure_id: i64,
) -> Result<(), ApiError> {
    let meta = state.structures.get_structure_meta(structure_id).await?;
    if meta.level == StructureLevel::Template {
        require_admin_or_root(claims)?;
    }
    Ok(())
}

/// `POST /api/structures` — create a structure.
///
/// There is no target structure yet, so the gate reads the *requested*
/// level: creating a template requires admin/root.
#[utoipa::path(
    post, path = "/", tag = "structures",
    operation_id = "structures_create_structure",
    request_body = dto::CreateStructureRequest,
    responses(
        (status = 201, description = "Structure created", body = dto::StructureViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required to create a template", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project or template not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "The project already has a structure", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_structure(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateStructureRequest>,
) -> Result<(StatusCode, Json<dto::StructureViewResponse>), ApiError> {
    let level = match req.level {
        dto::StructureLevelResponse::Template => StructureLevel::Template,
        dto::StructureLevelResponse::Project => StructureLevel::Project,
    };
    if level == StructureLevel::Template {
        require_admin_or_root(&claims)?;
    }
    let view = state
        .structures
        .create_structure(apis::structures::CreateStructureRequest {
            name: req.name,
            rootpath: req.rootpath,
            level,
            project_code: req.project_code,
            template_id: req.template_id,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/structures` — list structures.
#[utoipa::path(
    get, path = "/", tag = "structures",
    operation_id = "structures_list_structures",
    params(
        ("level" = Option<dto::StructureLevelResponse>, Query, description = "Restrict to one kind of structure"),
        ("projectCode" = Option<String>, Query, description = "Restrict to one project"),
    ),
    responses(
        (status = 200, description = "Structures list", body = dto::StructureListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_structures(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(query): Query<dto::ListStructuresQuery>,
) -> Result<Json<dto::StructureListResponse>, ApiError> {
    let level = query.level.map(|l| match l {
        dto::StructureLevelResponse::Template => StructureLevel::Template,
        dto::StructureLevelResponse::Project => StructureLevel::Project,
    });
    let out = state
        .structures
        .list_structures(apis::structures::ListStructuresRequest {
            level,
            project_code: query.project_code,
        })
        .await?;
    Ok(Json(dto::StructureListResponse {
        structures: out.structures.into_iter().map(Into::into).collect(),
    }))
}

/// `GET /api/structures/{id}` — fetch a structure with its effective tree.
#[utoipa::path(
    get, path = "/{id}", tag = "structures",
    operation_id = "structures_get_structure",
    params(
        ("id" = i64, Path, description = "Structure id"),
    ),
    responses(
        (status = 200, description = "Structure found", body = dto::StructureViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_structure(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::StructureViewResponse>, ApiError> {
    let view = state.structures.get_structure(id).await?;
    Ok(Json(view.into()))
}

/// `GET /api/structures/{id}/meta` — the structure's identity and level.
#[utoipa::path(
    get, path = "/{id}/meta", tag = "structures",
    operation_id = "structures_get_structure_meta",
    params(
        ("id" = i64, Path, description = "Structure id"),
    ),
    responses(
        (status = 200, description = "Structure found", body = dto::StructureMetaResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_structure_meta(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::StructureMetaResponse>, ApiError> {
    let meta = state.structures.get_structure_meta(id).await?;
    Ok(Json(meta.into()))
}

/// `PUT /api/structures/{id}` — rename a structure.
#[utoipa::path(
    put, path = "/{id}", tag = "structures",
    operation_id = "structures_rename_structure",
    params(
        ("id" = i64, Path, description = "Structure id"),
    ),
    request_body = dto::RenameStructureRequest,
    responses(
        (status = 200, description = "Structure renamed", body = dto::StructureViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required to change a template", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn rename_structure(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::RenameStructureRequest>,
) -> Result<Json<dto::StructureViewResponse>, ApiError> {
    require_template_write_permission(&state, &claims, id).await?;
    let view = state
        .structures
        .rename_structure(apis::structures::RenameStructureRequest {
            id,
            name: req.name,
            rootpath: req.rootpath,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/structures/{id}` — delete a structure.
#[utoipa::path(
    delete, path = "/{id}", tag = "structures",
    operation_id = "structures_delete_structure",
    params(
        ("id" = i64, Path, description = "Structure id"),
    ),
    responses(
        (status = 204, description = "Structure deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required to delete a template", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "The template is still in use", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_structure(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    require_template_write_permission(&state, &claims, id).await?;
    state.structures.delete_structure(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/structures/{id}/tree` — the effective tree alone.
#[utoipa::path(
    get, path = "/{id}/tree", tag = "structures",
    operation_id = "structures_build_structure",
    params(
        ("id" = i64, Path, description = "Structure id"),
    ),
    responses(
        (status = 200, description = "The effective tree", body = dto::StructureTreeResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn build_structure(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::StructureTreeResponse>, ApiError> {
    let tree = state.structures.build_structure(id).await?;
    Ok(Json(tree.into()))
}

/// `GET /api/structures/{id}/nodes` — the structure's own records.
///
/// This is where a client learns the `node_id` of a hidden node, so it
/// can undo a hide.
#[utoipa::path(
    get, path = "/{id}/nodes", tag = "structures",
    operation_id = "structures_list_own_nodes",
    params(
        ("id" = i64, Path, description = "Structure id"),
    ),
    responses(
        (status = 200, description = "The structure's own records", body = dto::OwnNodeListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_own_nodes(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::OwnNodeListResponse>, ApiError> {
    let out = state.structures.list_own_nodes(id).await?;
    Ok(Json(dto::OwnNodeListResponse {
        nodes: out.nodes.into_iter().map(Into::into).collect(),
    }))
}

/// `POST /api/structures/{id}/nodes` — place a node under a parent.
#[utoipa::path(
    post, path = "/{id}/nodes", tag = "structures",
    operation_id = "structures_add_node",
    params(
        ("id" = i64, Path, description = "Structure id"),
    ),
    request_body = dto::AddNodeRequest,
    responses(
        (status = 201, description = "Node created", body = dto::NodeViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required to change a template", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure or parent node not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "The name is already used under this parent", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn add_node(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::AddNodeRequest>,
) -> Result<(StatusCode, Json<dto::NodeViewResponse>), ApiError> {
    require_template_write_permission(&state, &claims, id).await?;
    let node = state
        .structures
        .add_node(apis::structures::AddNodeRequest {
            structure_id: id,
            entry_name: req.entry_name,
            description: req.description,
            directory: req.directory,
            parent: req.parent,
            reference: req.reference,
            migration: req.migration.map(|m| apis::structures::MigrationView {
                directory: m.directory,
                filter: m.filter,
            }),
            hidden: req.hidden,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(node.into())))
}

/// `PUT /api/structures/{id}/nodes/{node_id}` — replace a node.
#[utoipa::path(
    put, path = "/{id}/nodes/{node_id}", tag = "structures",
    operation_id = "structures_update_node",
    params(
        ("id" = i64, Path, description = "Structure id"),
        ("node_id" = String, Path, description = "Node id"),
    ),
    request_body = dto::UpdateNodeRequest,
    responses(
        (status = 200, description = "Node updated", body = dto::NodeViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required to change a template", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure or node not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "The name is already used under this parent", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_node(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::StructureNodePath {
        id: structure_id,
        node_id,
    }): Path<dto::StructureNodePath>,
    Json(req): Json<dto::UpdateNodeRequest>,
) -> Result<Json<dto::NodeViewResponse>, ApiError> {
    require_template_write_permission(&state, &claims, structure_id).await?;
    let node = state
        .structures
        .update_node(apis::structures::UpdateNodeRequest {
            structure_id,
            node_id,
            entry_name: req.entry_name,
            description: req.description,
            directory: req.directory,
            parent: req.parent,
            reference: req.reference,
            migration: req.migration.map(|m| apis::structures::MigrationView {
                directory: m.directory,
                filter: m.filter,
            }),
            hidden: req.hidden,
        })
        .await?;
    Ok(Json(node.into()))
}

/// `DELETE /api/structures/{id}/nodes/{node_id}` — remove a node.
#[utoipa::path(
    delete, path = "/{id}/nodes/{node_id}", tag = "structures",
    operation_id = "structures_delete_node",
    params(
        ("id" = i64, Path, description = "Structure id"),
        ("node_id" = String, Path, description = "Node id"),
    ),
    responses(
        (status = 204, description = "Node removed"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required to change a template", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Structure or node not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_node(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::StructureNodePath {
        id: structure_id,
        node_id,
    }): Path<dto::StructureNodePath>,
) -> Result<StatusCode, ApiError> {
    require_template_write_permission(&state, &claims, structure_id).await?;
    state
        .structures
        .delete_node(apis::structures::DeleteNodeRequest {
            structure_id,
            node_id,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    //! Handler tests for `/api/structures`.
    //!
    //! `MockStructureService` answers every service method
    //! successfully and `MockAuth` accepts any bearer token with a
    //! configurable role, so the tests exercise routing, extraction and
    //! the auth gate rather than behaviour.
    //!
    //! `all_routes_dispatch_to_their_handler` drives **every** route
    //! with a concrete path and asserts the status the handler
    //! returns. That is the test that catches a route whose path
    //! label does not match the extractor's field name — a mismatch
    //! that leaves the OpenAPI document looking perfectly correct.

    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::state::test_support::{
        NullCrfService, NullDomainModelService, NullMissionService, NullTerminologyService,
        NullUserService,
    };

    use apis::auth::{
        AuthApiError, AuthClaims as ApisAuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };
    use apis::structures::{
        AddNodeRequest, CreateStructureRequest, DeleteNodeRequest, ListStructuresRequest,
        RenameStructureRequest, StructureApiError, StructureLevel, StructureService,
        UpdateNodeRequest,
    };

    // ---- mock auth ----------------------------------------------------

    /// An auth service whose `verify` accepts any non-empty token and
    /// answers with the given role.
    #[derive(Clone)]
    pub struct MockAuth {
        role: apis::user::Role,
    }

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
                role: self.role,
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

    // ---- mock structures service --------------------------------------

    /// Every method answers successfully, so a failing route can only
    /// be a routing or extraction failure.
    #[derive(Clone, Default)]
    pub struct MockStructureService {
        /// The level `get_structure_meta` reports.
        pub level: Option<StructureLevel>,
        /// The root path the last `rename_structure` request carried.
        /// `None` means the request did not carry one.
        pub updated_rootpath: Arc<std::sync::Mutex<Option<Option<String>>>>,
    }

    fn structure_view() -> apis::structures::StructureView {
        apis::structures::StructureView {
            id: 1,
            name: "Standard".into(),
            rootpath: "/abs/root".into(),
            level: StructureLevel::Template,
            project_code: None,
            template_id: None,
            nodes: Vec::new(),
        }
    }

    fn node_view() -> apis::structures::NodeView {
        apis::structures::NodeView {
            node_id: "n1".into(),
            description: String::new(),
            entry_name: "raw".into(),
            directory: true,
            parent: None,
            reference: None,
            migration: None,
            children: Vec::new(),
        }
    }

    #[async_trait]
    impl StructureService for MockStructureService {
        async fn create_structure(
            &self,
            _req: CreateStructureRequest,
        ) -> Result<apis::structures::StructureView, StructureApiError> {
            Ok(structure_view())
        }
        async fn get_structure(
            &self,
            _id: i64,
        ) -> Result<apis::structures::StructureView, StructureApiError> {
            Ok(structure_view())
        }
        async fn get_structure_meta(
            &self,
            _id: i64,
        ) -> Result<apis::structures::StructureMetaView, StructureApiError> {
            Ok(apis::structures::StructureMetaView {
                id: 1,
                name: "Standard".into(),
                rootpath: "/abs/root".into(),
                level: self.level.unwrap_or(StructureLevel::Template),
                project_code: None,
                template_id: None,
            })
        }
        async fn list_structures(
            &self,
            _req: ListStructuresRequest,
        ) -> Result<apis::structures::StructureList, StructureApiError> {
            Ok(apis::structures::StructureList {
                structures: vec![apis::structures::StructureSummaryView {
                    id: 1,
                    name: "Standard".into(),
                    rootpath: "/abs/root".into(),
                    level: StructureLevel::Template,
                    project_code: None,
                    template_id: None,
                }],
            })
        }
        async fn rename_structure(
            &self,
            req: RenameStructureRequest,
        ) -> Result<apis::structures::StructureView, StructureApiError> {
            *self.updated_rootpath.lock().unwrap() = Some(req.rootpath);
            Ok(structure_view())
        }
        async fn delete_structure(&self, _id: i64) -> Result<(), StructureApiError> {
            Ok(())
        }
        async fn add_node(
            &self,
            _req: AddNodeRequest,
        ) -> Result<apis::structures::NodeView, StructureApiError> {
            Ok(node_view())
        }
        async fn update_node(
            &self,
            _req: UpdateNodeRequest,
        ) -> Result<apis::structures::NodeView, StructureApiError> {
            Ok(node_view())
        }
        async fn delete_node(&self, _req: DeleteNodeRequest) -> Result<(), StructureApiError> {
            Ok(())
        }
        async fn list_own_nodes(
            &self,
            _structure_id: i64,
        ) -> Result<apis::structures::OwnNodeList, StructureApiError> {
            Ok(apis::structures::OwnNodeList { nodes: Vec::new() })
        }
        async fn build_structure(
            &self,
            _id: i64,
        ) -> Result<apis::structures::StructureTreeView, StructureApiError> {
            Ok(apis::structures::StructureTreeView {
                structure_id: 1,
                nodes: Vec::new(),
            })
        }
    }

    // ---- state + router ------------------------------------------------

    fn test_state(service: MockStructureService, role: apis::user::Role) -> AppState {
        AppState {
            auth: Arc::new(MockAuth { role }) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(crate::state::test_support::NullProjectService)
                as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(NullMissionService) as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(NullCrfService) as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(crate::state::test_support::NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: Arc::new(service) as Arc<dyn apis::structures::StructureService>,
            log_ingestor: crate::state::test_support::unused_log_ingestor(),
        }
    }

    /// The **real** top-level router, so the `/api/structures` mount
    /// and every path parameter are exercised exactly as a client
    /// would meet them.
    fn app(state: AppState) -> Router {
        crate::transport::http::router::router(state)
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(uri)
            .header("Authorization", "Bearer token")
            .body(Body::empty())
            .unwrap()
    }

    fn post(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("Authorization", "Bearer token")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn put(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("PUT")
            .uri(uri)
            .header("Authorization", "Bearer token")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn delete(uri: &str) -> Request<Body> {
        Request::builder()
            .method("DELETE")
            .uri(uri)
            .header("Authorization", "Bearer token")
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn all_routes_dispatch_to_their_handler() {
        // (method, uri, request body, expected status)
        let cases: Vec<(&str, &str, Option<&str>, AxStatus)> = vec![
            (
                "POST",
                "/api/structures",
                Some(r#"{"name":"Standard","rootpath":"/abs/root","level":"template"}"#),
                AxStatus::CREATED,
            ),
            ("GET", "/api/structures", None, AxStatus::OK),
            ("GET", "/api/structures/1", None, AxStatus::OK),
            ("GET", "/api/structures/1/meta", None, AxStatus::OK),
            (
                "PUT",
                "/api/structures/1",
                Some(r#"{"name":"Renamed"}"#),
                AxStatus::OK,
            ),
            ("DELETE", "/api/structures/1", None, AxStatus::NO_CONTENT),
            ("GET", "/api/structures/1/tree", None, AxStatus::OK),
            ("GET", "/api/structures/1/nodes", None, AxStatus::OK),
            (
                "POST",
                "/api/structures/1/nodes",
                Some(r#"{"entryName":"raw","description":"","directory":true,"parent":null}"#),
                AxStatus::CREATED,
            ),
            (
                "PUT",
                "/api/structures/1/nodes/n1",
                Some(r#"{"entryName":"raw","description":"","directory":true,"parent":null}"#),
                AxStatus::OK,
            ),
            (
                "DELETE",
                "/api/structures/1/nodes/n1",
                None,
                AxStatus::NO_CONTENT,
            ),
        ];

        for (method, uri, body, expected) in cases {
            let state = test_state(MockStructureService::default(), apis::user::Role::Admin);
            let app = app(state);
            let request = match (method, body) {
                ("GET", _) => get(uri),
                ("POST", Some(b)) => post(uri, b),
                ("PUT", Some(b)) => put(uri, b),
                ("DELETE", _) => delete(uri),
                _ => panic!("unhandled case {method} {uri}"),
            };
            let response = app.oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                expected,
                "{method} {uri} answered {}",
                response.status()
            );
        }
    }

    #[tokio::test]
    async fn a_template_write_by_a_general_user_returns_403() {
        let state = test_state(
            MockStructureService {
                level: Some(StructureLevel::Template),
                ..Default::default()
            },
            apis::user::Role::General,
        );
        let app = app(state);
        let response = app.oneshot(delete("/api/structures/1")).await.unwrap();
        assert_eq!(response.status(), AxStatus::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_project_write_by_a_general_user_succeeds() {
        let state = test_state(
            MockStructureService {
                level: Some(StructureLevel::Project),
                ..Default::default()
            },
            apis::user::Role::General,
        );
        let app = app(state);
        let response = app.oneshot(delete("/api/structures/1")).await.unwrap();
        assert_eq!(response.status(), AxStatus::NO_CONTENT);
    }

    #[tokio::test]
    async fn creating_a_template_as_a_general_user_returns_403() {
        let state = test_state(MockStructureService::default(), apis::user::Role::General);
        let app = app(state);
        let response = app
            .oneshot(post(
                "/api/structures",
                r#"{"name":"Standard","rootpath":"/abs/root","level":"template"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::FORBIDDEN);
    }

    #[tokio::test]
    async fn creating_a_project_structure_as_a_general_user_succeeds() {
        let state = test_state(MockStructureService::default(), apis::user::Role::General);
        let app = app(state);
        let response = app
            .oneshot(post(
                "/api/structures",
                r#"{"name":"Study","rootpath":"/abs/root","level":"project","projectCode":"ABC","templateId":1}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::CREATED);
    }

    #[tokio::test]
    async fn a_read_by_an_unauthenticated_caller_returns_401() {
        let state = test_state(MockStructureService::default(), apis::user::Role::Admin);
        let app = app(state);
        let request = Request::builder()
            .method("GET")
            .uri("/api/structures")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_create_without_a_rootpath_is_rejected() {
        let state = test_state(MockStructureService::default(), apis::user::Role::Admin);
        let app = app(state);
        let response = app
            .oneshot(post(
                "/api/structures",
                r#"{"name":"Standard","level":"template"}"#,
            ))
            .await
            .unwrap();
        // the body does not deserialize -- axum rejects it before the
        // handler runs
        assert_eq!(response.status(), AxStatus::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn an_update_without_a_root_path_leaves_the_stored_one_alone() {
        let service = MockStructureService::default();
        let state = test_state(service.clone(), apis::user::Role::Admin);
        let app = app(state);
        let response = app
            .oneshot(put("/api/structures/1", r#"{"name":"Renamed"}"#))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        // the handler forwarded "no root path", not an empty one
        assert_eq!(*service.updated_rootpath.lock().unwrap(), Some(None));
    }

    #[tokio::test]
    async fn an_update_with_a_null_root_path_leaves_the_stored_one_alone() {
        let service = MockStructureService::default();
        let state = test_state(service.clone(), apis::user::Role::Admin);
        let app = app(state);
        let response = app
            .oneshot(put(
                "/api/structures/1",
                r#"{"name":"Renamed","rootpath":null}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        assert_eq!(*service.updated_rootpath.lock().unwrap(), Some(None));
    }
}
