//! [`submit`] handler for `POST /api/log-ingest/submit`.

use axum::Json;
use axum::extract::State;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto::{self, LogIngestRequest, LogIngestResponse};
use crate::transport::http::error::ApiError;

/// Maximum number of entries per request. Matches
/// `LogIngestorConfig`'s default `channel_capacity` order of
/// magnitude — one runaway client cannot fill the bounded queue
/// in a single shot.
pub(crate) const MAX_ENTRIES_PER_REQUEST: usize = 10_000;

#[utoipa::path(
    post,
    path = "/submit",
    tag = "log-ingest",
    operation_id = "log_ingest_submit",
    request_body = dto::LogIngestRequest,
    responses(
        (status = 200, description = "Batch accepted", body = dto::LogIngestResponse),
        (status = 400, description = "Validation failed (empty / over-cap entries, malformed JSON)", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid bearer", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate batch id within cache TTL", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Ingestor init / writer panic / writer timeout", body = crate::transport::http::error::ErrorBody),
        (status = 503, description = "Channel full / ingestor shut down", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn submit(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<LogIngestRequest>,
) -> Result<Json<LogIngestResponse>, ApiError> {
    if req.entries.is_empty() {
        return Err(ApiError::Validation(
            "entries must contain at least one line".into(),
        ));
    }
    if req.entries.len() > MAX_ENTRIES_PER_REQUEST {
        return Err(ApiError::Validation(format!(
            "entries must contain at most {MAX_ENTRIES_PER_REQUEST} lines (got {})",
            req.entries.len()
        )));
    }

    let accepted = req.entries.len();
    let batch_id = req.batch_id.clone();
    state.log_ingestor.submit(req.batch_id, &req.entries)?;

    Ok(Json(LogIngestResponse { batch_id, accepted }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, test_support};
    use crate::transport::http::dto;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use logging_utils::LogIngestor;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn router(state: AppState) -> Router {
        Router::new()
            .route("/api/log-ingest/submit", post(submit))
            .with_state(state)
    }

    fn build_state(dir: &std::path::Path) -> AppState {
        let cfg = logging_utils::LogIngestorConfig::builder()
            .log_dir(dir.to_path_buf())
            .file_name_prefix("test.log".into())
            .build();
        let log_ingestor = Arc::new(LogIngestor::new(cfg).expect("ingestor init"));
        AppState {
            auth: Arc::new(test_support::NullAuthService) as Arc<dyn apis::auth::AuthService>,
            user: Arc::new(test_support::NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(test_support::NullProjectService)
                as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(test_support::NullCrfService) as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(test_support::NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            log_ingestor,
        }
    }

    fn make_request(body: Vec<u8>, bearer: Option<&str>) -> Request<Body> {
        let mut b = Request::builder()
            .method("POST")
            .uri("/api/log-ingest/submit")
            .header("content-type", "application/json");
        if let Some(t) = bearer {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        b.body(Body::from(body)).unwrap()
    }

    async fn read_body(resp: axum::response::Response) -> Vec<u8> {
        axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    fn file_contents(dir: &std::path::Path) -> Vec<String> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            if entry.file_name().to_string_lossy().starts_with("test.log") {
                let s = std::fs::read_to_string(entry.path()).unwrap();
                out.extend(s.lines().map(String::from));
            }
        }
        out.sort();
        out
    }

    #[tokio::test]
    async fn submit_valid_batch_writes_to_file_and_returns_accepted() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-1".into(),
                entries: vec!["alpha".into(), "beta".into()],
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: dto::LogIngestResponse = serde_json::from_slice(&read_body(resp).await).unwrap();
        assert_eq!(body.batch_id, "b-1");
        assert_eq!(body.accepted, 2);

        assert_eq!(
            file_contents(tmp.path()),
            vec!["alpha".to_string(), "beta".to_string()]
        );
    }

    #[tokio::test]
    async fn submit_empty_entries_returns_400() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-empty".into(),
                entries: vec![],
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert!(
            file_contents(tmp.path()).is_empty(),
            "spec: empty entries → 400 with no file write",
        );
    }

    #[tokio::test]
    async fn submit_at_boundary_10000_entries_is_accepted() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let entries: Vec<String> = (0..10_000).map(|i| format!("line-{i}")).collect();
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-10000".into(),
                entries,
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn submit_over_cap_10001_entries_returns_400() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let entries: Vec<String> = (0..10_001).map(|i| format!("line-{i}")).collect();
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-10001".into(),
                entries,
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert!(
            file_contents(tmp.path()).is_empty(),
            "spec: over-cap entries → 400 with no file write",
        );
    }

    #[tokio::test]
    async fn submit_duplicate_batch_id_returns_409() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());

        let req1 = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-dup".into(),
                entries: vec!["first".into()],
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp1 = router(state.clone()).oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        let req2 = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-dup".into(),
                entries: vec!["second".into()],
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp2 = router(state).oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
        assert_eq!(
            file_contents(tmp.path()),
            vec!["first".to_string()],
            "spec: duplicate batch_id → 409 with no file write for the second batch",
        );
    }

    #[tokio::test]
    async fn submit_missing_bearer_returns_401() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-401".into(),
                entries: vec!["x".into()],
            })
            .unwrap(),
            None,
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn submit_channel_full_returns_503() {
        // Review Focus #1: a small channel against a slow writer
        // surfaces `ChannelFull(1)` → 503.
        let tmp = tempfile::tempdir().unwrap();
        let cfg = logging_utils::LogIngestorConfig::builder()
            .log_dir(tmp.path().to_path_buf())
            .file_name_prefix("full.log".into())
            .channel_capacity(1)
            .shutdown_deadline(std::time::Duration::from_secs(2))
            .build();
        let log_ingestor = Arc::new(LogIngestor::new(cfg).expect("ingestor init"));
        let state = AppState {
            auth: Arc::new(test_support::NullAuthService) as Arc<dyn apis::auth::AuthService>,
            user: Arc::new(test_support::NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(test_support::NullProjectService)
                as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(test_support::NullCrfService) as Arc<dyn apis::crf::CrfService>,
            sdtm_project: Arc::new(test_support::NullSdtmProjectService)
                as Arc<dyn apis::sdtm_project::SdtmProjectService>,
            structures: std::sync::Arc::new(crate::state::test_support::NullStructureService)
                as std::sync::Arc<dyn apis::structures::StructureService>,
            log_ingestor: log_ingestor.clone(),
        };

        let big: Vec<String> = (0..5_000).map(|i| format!("entry-{i:06}")).collect();
        let mut observed_503 = false;
        for i in 0..50 {
            let req = make_request(
                serde_json::to_vec(&dto::LogIngestRequest {
                    batch_id: format!("b-{i}"),
                    entries: big.clone(),
                })
                .unwrap(),
                Some("any-token"),
            );
            let resp = router(state.clone()).oneshot(req).await.unwrap();
            if resp.status() == StatusCode::SERVICE_UNAVAILABLE {
                observed_503 = true;
                break;
            }
            assert_eq!(resp.status(), StatusCode::OK);
        }
        assert!(
            observed_503,
            "ChannelFull(1) must be reachable with capacity 1",
        );
    }
}
