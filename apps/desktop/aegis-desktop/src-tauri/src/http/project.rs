//! Project CRUD.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberDataRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDataRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagViewResponse {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummaryViewResponse {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberViewResponse {
    pub leaders: Vec<UserSummaryViewResponse>,
    pub workers: Vec<UserSummaryViewResponse>,
}

/// Wire-level project language code. Two variants; per-variant
/// `#[serde(rename = ...)]` so the wire codes are exactly `"en"` and
/// `"zh-CN"` — matching the i18n `Locale` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectLanguage {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

/// Wire-level request body for the SDTM-IG version pointer a
/// project targets. Mirrors the server's
/// `apps/server/aegis-server/src/transport/http/dto.rs::ModelVersionRequest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionRequest {
    pub version_id: i64,
    pub version_name: String,
}

/// Wire-level projection of the SDTM-IG version pointer. Mirrors the
/// server's `apps/server/aegis-server/src/transport/http/dto.rs::ModelVersionResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionResponse {
    pub version_id: i64,
    pub version_name: String,
}

/// Wire-level request body for the SDTM controlled-terminology
/// version pointer a project pins. Mirrors the server's
/// `apps/server/aegis-server/src/transport/http/dto.rs::TerminologyVersionRequest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionRequest {
    pub version_id: i64,
    pub version_name: String,
}

/// Wire-level projection of the SDTM controlled-terminology version
/// pointer. Mirrors the server's
/// `apps/server/aegis-server/src/transport/http/dto.rs::TerminologyVersionResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionResponse {
    pub version_id: i64,
    pub version_name: String,
}

/// Wire-level request body for a project's configuration.
/// `language` and `tags` are individually optional / empty-skippable;
/// `sdtmig` and `sdtm_terminology` are skipped on serialize when
/// absent so a present-but-empty config round-trips as `{}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationDataRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtmig: Option<ModelVersionRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtm_terminology: Option<TerminologyVersionRequest>,
}

/// Wire-level projection of a project's configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationViewResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagViewResponse>,
    pub sdtmig: Option<ModelVersionResponse>,
    pub sdtm_terminology: Option<TerminologyVersionResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub configurations: ProjectConfigurationViewResponse,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

/// Wire-level projection of a project branch.
///
/// The server dropped the `active` flag from the branch row, so the wire
/// carries no such field and the desktop models none: which branch is
/// active is desktop-local state, not something the server knows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBranchViewResponse {
    pub id: i32,
    pub project_id: i32,
    pub branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Wire-level request body for `POST /api/project/{code}/branches`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectBranchRequest {
    pub branch: String,
}

/// Wire-level request body for `PATCH /api/project/{code}/branches/{id}`.
///
/// An absent `branch` leaves the name alone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectBranchRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// Server wire shape for the branch list. The envelope is unwrapped by
/// [`list_branches`], exactly as the project list is.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectBranchListWire {
    branches: Vec<ProjectBranchViewResponse>,
}

pub async fn create(
    c: &HttpClient,
    body: CreateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/project", Some(&body))
        .await
}

pub async fn list(c: &HttpClient) -> Result<Vec<ProjectViewResponse>, ApiError> {
    let resp: ProjectListResponse = c
        .request(reqwest::Method::GET, "/api/project", None::<&()>)
        .await?;
    Ok(resp.projects)
}

pub async fn get_by_code(c: &HttpClient, code: &str) -> Result<ProjectViewResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/project/{code}"),
        None::<&()>,
    )
    .await
}

pub async fn update(
    c: &HttpClient,
    code: &str,
    body: UpdateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/project/{code}"),
        Some(&body),
    )
    .await
}

/// `GET /api/project/{code}/branches` — the project's branches.
///
/// The server wraps the list in a `{ "branches": [...] }` envelope; this
/// unwraps it so the caller sees a plain `Vec`, the same shape the
/// project list is returned in.
pub async fn list_branches(
    c: &HttpClient,
    code: &str,
) -> Result<Vec<ProjectBranchViewResponse>, ApiError> {
    let wire: ProjectBranchListWire = c
        .request(
            reqwest::Method::GET,
            &format!("/api/project/{code}/branches"),
            None::<&()>,
        )
        .await?;
    Ok(wire.branches)
}

/// `POST /api/project/{code}/branches` — create a branch.
pub async fn create_branch(
    c: &HttpClient,
    code: &str,
    body: CreateProjectBranchRequest,
) -> Result<ProjectBranchViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/project/{code}/branches"),
        Some(&body),
    )
    .await
}

/// `PATCH /api/project/{code}/branches/{id}` — rename a branch.
pub async fn update_branch(
    c: &HttpClient,
    code: &str,
    id: i32,
    body: UpdateProjectBranchRequest,
) -> Result<ProjectBranchViewResponse, ApiError> {
    c.request(
        reqwest::Method::PATCH,
        &format!("/api/project/{code}/branches/{id}"),
        Some(&body),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore, TokenStore};

    #[tokio::test]
    async fn list_returns_projects() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/project"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "projects": [{
                            "id": 1, "code": "p", "description": "",
                            "configurations": {
                                "language": "en",
                                "tags": [{ "key": "Product", "value": "DEMO-001" }],
                                "sdtmig": { "versionId": 2, "versionName": "SDTMIG v3.2" }
                            },
                            "members": { "leaders": [], "workers": [] },
                            "unblindMembers": { "leaders": [], "workers": [] },
                            "active": true,
                            "createdAt": "2026-01-01T00:00:00Z",
                            "updatedAt": "2026-01-02T00:00:00Z"
                        }]
                    }))),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);
        let projects = list(&c).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].code, "p");
        assert_eq!(
            projects[0].configurations.language,
            Some(ProjectLanguage::English)
        );
        assert_eq!(projects[0].configurations.tags.len(), 1);
        assert_eq!(projects[0].configurations.tags[0].key, "Product");
        // sdtmig round-trips: the wire JSON carries `versionId` /
        // `versionName` (camelCase) which deserialize back into the
        // snake_case Rust fields.
        let sdtmig = projects[0]
            .configurations
            .sdtmig
            .as_ref()
            .expect("sdtmig present");
        assert_eq!(sdtmig.version_id, 2);
        assert_eq!(sdtmig.version_name, "SDTMIG v3.2");
    }

    #[test]
    fn update_skips_none_fields() {
        let body = UpdateProjectRequest {
            active: Some(false),
            ..Default::default()
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"active":false}"#);
        assert!(!j.contains("configurations"));
    }

    #[test]
    fn project_language_serializes_to_wire_codes() {
        assert_eq!(
            serde_json::to_string(&ProjectLanguage::English).unwrap(),
            "\"en\""
        );
        assert_eq!(
            serde_json::to_string(&ProjectLanguage::SimplifiedChinese).unwrap(),
            "\"zh-CN\""
        );
    }

    #[test]
    fn project_configuration_data_request_omits_empty() {
        let body = ProjectConfigurationDataRequest::default();
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, "{}");

        let body = ProjectConfigurationDataRequest {
            language: Some(ProjectLanguage::SimplifiedChinese),
            tags: vec![TagDataRequest {
                key: "Product".into(),
                value: "DEMO-001".into(),
            }],
            sdtmig: None,
            sdtm_terminology: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(
            j,
            r#"{"language":"zh-CN","tags":[{"key":"Product","value":"DEMO-001"}]}"#
        );

        // sdtmig is camelCase on the wire and skipped on serialize
        // when None.
        let body = ProjectConfigurationDataRequest {
            language: None,
            tags: vec![],
            sdtmig: Some(ModelVersionRequest {
                version_id: 3,
                version_name: "SDTMIG v3.3".into(),
            }),
            sdtm_terminology: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(
            j,
            r#"{"sdtmig":{"versionId":3,"versionName":"SDTMIG v3.3"}}"#
        );
    }

    #[test]
    fn model_version_request_round_trips() {
        let body = ModelVersionRequest {
            version_id: 7,
            version_name: "SDTMIG v3.4".into(),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"versionId":7,"versionName":"SDTMIG v3.4"}"#);
        let parsed: ModelVersionRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(parsed, body);
    }

    #[test]
    fn terminology_version_request_round_trips() {
        let body = TerminologyVersionRequest {
            version_id: 11,
            version_name: "2024-03-29".into(),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"versionId":11,"versionName":"2024-03-29"}"#);
        let parsed: TerminologyVersionRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(parsed, body);
    }

    #[test]
    fn project_configuration_data_request_serialises_sdtm_terminology() {
        let body = ProjectConfigurationDataRequest {
            language: None,
            tags: vec![],
            sdtmig: None,
            sdtm_terminology: Some(TerminologyVersionRequest {
                version_id: 11,
                version_name: "2024-03-29".into(),
            }),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(
            j,
            r#"{"sdtmTerminology":{"versionId":11,"versionName":"2024-03-29"}}"#
        );
    }

    #[test]
    fn project_member_data_request_omits_empty_arrays() {
        let body = ProjectMemberDataRequest::default();
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, "{}");
        let body = ProjectMemberDataRequest {
            leaders: vec!["a".into()],
            workers: vec![],
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"leaders":["a"]}"#);
    }

    #[test]
    fn tag_data_request_round_trips() {
        let body = TagDataRequest {
            key: "Product".into(),
            value: "DEMO-001".into(),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"key":"Product","value":"DEMO-001"}"#);
        let parsed: TagDataRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(parsed, body);
    }

    // -- project branches ------------------------------------------------

    /// A branch view parses the server's wire shape. The server dropped
    /// the `active` flag from the branch row (commit `1ab41d2`), so the
    /// wire carries no such field and the desktop models none.
    #[test]
    fn project_branch_view_parses_the_server_wire_shape() {
        let raw = r#"{
            "id": 4,
            "projectId": 9,
            "branch": "2026Q1",
            "createdAt": "2026-01-02T03:04:05Z",
            "updatedAt": "2026-01-02T03:04:05Z"
        }"#;
        let view: ProjectBranchViewResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(view.id, 4);
        assert_eq!(view.project_id, 9);
        assert_eq!(view.branch, "2026Q1");
    }

    /// An absent name is skipped on the wire rather than sent as null.
    #[test]
    fn update_project_branch_request_skips_absent_fields() {
        let body = UpdateProjectBranchRequest {
            branch: Some("2026Q2".into()),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"branch":"2026Q2"}"#);

        let body = UpdateProjectBranchRequest { branch: None };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, "{}");
    }

    /// The server wraps the list in a `{ "branches": [...] }` envelope;
    /// the desktop unwraps it, exactly as `list` does for projects.
    #[tokio::test]
    async fn list_branches_unwraps_the_envelope() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/project/pr1/branches"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "branches": [{
                            "id": 4,
                            "projectId": 9,
                            "branch": "2026Q1",
                            "createdAt": "2026-01-02T03:04:05Z",
                            "updatedAt": "2026-01-02T03:04:05Z"
                        }]
                    }))),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);
        let branches = list_branches(&c, "pr1").await.unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].branch, "2026Q1");
        assert_eq!(branches[0].project_id, 9);
    }

    #[tokio::test]
    async fn create_branch_posts_the_branch_name() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/project/pr1/branches"))
                    .and(body_json(serde_json::json!({ "branch": "2026Q1" })))
                    .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                        "id": 4,
                        "projectId": 9,
                        "branch": "2026Q1",
                        "createdAt": "2026-01-02T03:04:05Z",
                        "updatedAt": "2026-01-02T03:04:05Z"
                    }))),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);
        let view = create_branch(
            &c,
            "pr1",
            CreateProjectBranchRequest {
                branch: "2026Q1".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(view.id, 4);
        assert_eq!(view.branch, "2026Q1");
    }

    #[tokio::test]
    async fn update_branch_patches_the_branch_name() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server
            .register(
                Mock::given(method("PATCH"))
                    .and(path("/api/project/pr1/branches/4"))
                    .and(body_json(serde_json::json!({ "branch": "2026Q2" })))
                    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "id": 4,
                        "projectId": 9,
                        "branch": "2026Q2",
                        "createdAt": "2026-01-02T03:04:05Z",
                        "updatedAt": "2026-01-02T03:04:05Z"
                    }))),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);
        let view = update_branch(
            &c,
            "pr1",
            4,
            UpdateProjectBranchRequest {
                branch: Some("2026Q2".into()),
            },
        )
        .await
        .unwrap();
        assert_eq!(view.id, 4);
        assert_eq!(view.branch, "2026Q2");
    }
}
