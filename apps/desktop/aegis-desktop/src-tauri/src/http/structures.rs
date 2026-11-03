//! Directory-structure HTTP adapters. Wire DTOs mirror the server's
//! `apps/server/aegis-server/src/transport/http/dto.rs` §structures.

use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;

/// Which kind of structure this is.
///
/// Mirrors the server's `StructureLevelResponse`. The wire form is
/// lowercase (`"template"`, `"project"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StructureLevelResponse {
    Template,
    Project,
}

/// What to copy into a directory once it has been created.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationViewResponse {
    pub directory: String,
    pub filter: Vec<String>,
}

/// One node of the effective tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeViewResponse {
    pub node_id: String,
    pub description: String,
    pub path: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationViewResponse>,
    pub children: Vec<NodeViewResponse>,
}

/// A structure and its effective tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureViewResponse {
    pub id: i64,
    pub name: String,
    pub level: StructureLevelResponse,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
    pub nodes: Vec<NodeViewResponse>,
}

/// A structure's identity, without its tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureSummaryResponse {
    pub id: i64,
    pub name: String,
    pub level: StructureLevelResponse,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// The list envelope the server returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureListResponse {
    pub structures: Vec<StructureSummaryResponse>,
}

/// Place a node at a path. `parent` is always explicit: `None` means
/// "at the root".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddNodeRequest {
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationViewResponse>,
}

/// Replace a node's mutable attributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNodeRequest {
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationViewResponse>,
}

/// `GET /api/structures` — list structures, optionally filtered by level.
pub async fn list_structures(
    c: &HttpClient,
    level: Option<StructureLevelResponse>,
) -> Result<Vec<StructureSummaryResponse>, ApiError> {
    let mut url = "/api/structures".to_string();
    if let Some(l) = level {
        // Serialize the typed enum and strip the quotes, the same way
        // `http::mission` builds its `?kind=` query parameter.
        let level_str = serde_json::to_string(&l)
            .map_err(|e| ApiError::Parse {
                message: e.to_string(),
            })?
            .trim_matches('"')
            .to_string();
        url.push_str("?level=");
        url.push_str(&level_str);
    }
    let resp: StructureListResponse = c.request(reqwest::Method::GET, &url, None::<&()>).await?;
    Ok(resp.structures)
}

/// `GET /api/structures/{id}` — a structure with its effective tree.
pub async fn get_structure(c: &HttpClient, id: i64) -> Result<StructureViewResponse, ApiError> {
    let resp: StructureViewResponse = c
        .request(
            reqwest::Method::GET,
            &format!("/api/structures/{id}"),
            None::<&()>,
        )
        .await?;
    Ok(resp)
}

/// `POST /api/structures/{id}/nodes` — place a node at a path.
pub async fn add_node(
    c: &HttpClient,
    structure_id: i64,
    body: &AddNodeRequest,
) -> Result<NodeViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/structures/{structure_id}/nodes"),
        Some(body),
    )
    .await
}

/// `PUT /api/structures/{id}/nodes/{node_id}` — replace a node.
pub async fn update_node(
    c: &HttpClient,
    structure_id: i64,
    node_id: &str,
    body: &UpdateNodeRequest,
) -> Result<NodeViewResponse, ApiError> {
    c.request(
        reqwest::Method::PUT,
        &format!("/api/structures/{structure_id}/nodes/{node_id}"),
        Some(body),
    )
    .await
}

/// `DELETE /api/structures/{id}/nodes/{node_id}` — remove a node.
///
/// The server answers `204` with an empty body, so the adapter must not
/// try to JSON-decode it — `request::<Value>` would fail with
/// `decode_failed: EOF while parsing a value`. See
/// `http::mission::remove_assignee` for the same trap.
pub async fn delete_node(
    c: &HttpClient,
    structure_id: i64,
    node_id: &str,
) -> Result<(), ApiError> {
    let _ = c
        .request_bytes(
            reqwest::Method::DELETE,
            &format!("/api/structures/{structure_id}/nodes/{node_id}"),
            None::<&()>,
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::client::{HttpClient, MemoryStore, TokenStore};
    use serde_json::json;
    use std::sync::Arc;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// A client pointed at a fresh mock server, with an access token set.
    async fn mock_client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        HttpClient::new(server.uri(), store)
    }

    #[tokio::test]
    async fn list_structures_requests_the_level_filter() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/structures"))
                    .and(query_param("level", "template"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                        "structures": [{
                            "id": 3,
                            "name": "Standard",
                            "level": "template",
                            "projectCode": null,
                            "templateId": null
                        }]
                    }))),
            )
            .await;
        let c = mock_client(&server).await;

        let out = list_structures(&c, Some(StructureLevelResponse::Template))
            .await
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, 3);
        assert_eq!(out[0].level, StructureLevelResponse::Template);
    }

    #[tokio::test]
    async fn add_node_posts_the_exact_body_to_the_nodes_path() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/structures/7/nodes"))
                    .and(body_json(json!({
                        "path": "raw",
                        "description": "d",
                        "directory": true,
                        "parent": null,
                        "reference": null,
                        "migration": null
                    })))
                    .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                        "nodeId": "n9",
                        "description": "d",
                        "path": "raw",
                        "directory": true,
                        "parent": null,
                        "reference": null,
                        "migration": null,
                        "children": []
                    }))),
            )
            .await;
        let c = mock_client(&server).await;

        let body = AddNodeRequest {
            path: "raw".into(),
            description: "d".into(),
            directory: true,
            parent: None,
            reference: None,
            migration: None,
        };
        let node = add_node(&c, 7, &body).await.unwrap();
        assert_eq!(node.node_id, "n9");
    }

    #[tokio::test]
    async fn update_node_puts_the_exact_body_to_the_node_path() {
        let server = MockServer::start().await;
        server
            .register(
                Mock::given(method("PUT"))
                    .and(path("/api/structures/7/nodes/n1"))
                    .and(body_json(json!({
                        "path": "raw/data",
                        "description": "d",
                        "directory": true,
                        "parent": "n0",
                        "reference": null,
                        "migration": null
                    })))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                        "nodeId": "n1",
                        "description": "d",
                        "path": "raw/data",
                        "directory": true,
                        "parent": "n0",
                        "reference": null,
                        "migration": null,
                        "children": []
                    }))),
            )
            .await;
        let c = mock_client(&server).await;

        let body = UpdateNodeRequest {
            path: "raw/data".into(),
            description: "d".into(),
            directory: true,
            parent: Some("n0".into()),
            reference: None,
            migration: None,
        };
        let node = update_node(&c, 7, "n1", &body).await.unwrap();
        assert_eq!(node.node_id, "n1");
    }

    #[test]
    fn structure_level_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&StructureLevelResponse::Template).unwrap(),
            "\"template\""
        );
        assert_eq!(
            serde_json::to_string(&StructureLevelResponse::Project).unwrap(),
            "\"project\""
        );
    }

    #[test]
    fn node_view_response_parses_the_full_wire_shape() {
        let j = r#"{
            "nodeId": "n1",
            "description": "raw data",
            "path": "raw/data",
            "directory": true,
            "parent": "n0",
            "reference": null,
            "migration": { "directory": "/opt/sop/raw", "filter": ["*.sas7bdat"] },
            "children": [
                {
                    "nodeId": "n2",
                    "description": "",
                    "path": "raw/data/a.sas7bdat",
                    "directory": false,
                    "parent": "n1",
                    "reference": "/opt/sop/raw/a.sas7bdat",
                    "migration": null,
                    "children": []
                }
            ]
        }"#;
        let node: NodeViewResponse = serde_json::from_str(j).unwrap();
        assert_eq!(node.node_id, "n1");
        assert!(node.directory);
        assert_eq!(node.parent.as_deref(), Some("n0"));
        assert_eq!(node.migration.unwrap().filter, vec!["*.sas7bdat"]);
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].node_id, "n2");
        assert!(!node.children[0].directory);
    }

    #[test]
    fn add_node_request_serializes_camel_case_with_a_null_parent() {
        let body = AddNodeRequest {
            path: "raw".into(),
            description: "d".into(),
            directory: true,
            parent: None,
            reference: None,
            migration: None,
        };
        let j = serde_json::to_string(&body).unwrap();
        assert!(j.contains("\"parent\":null"), "got {j}");
        assert!(j.contains("\"directory\":true"), "got {j}");
        assert!(j.contains("\"path\":\"raw\""), "got {j}");
    }

    #[tokio::test]
    async fn delete_node_succeeds_on_204_no_body() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        server
            .register(
                Mock::given(method("DELETE"))
                    .and(path("/api/structures/7/nodes/n1"))
                    .respond_with(ResponseTemplate::new(204)),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);

        let result = delete_node(&c, 7, "n1").await;
        assert!(
            result.is_ok(),
            "delete_node must succeed on 204; got {result:?}"
        );
    }
}
