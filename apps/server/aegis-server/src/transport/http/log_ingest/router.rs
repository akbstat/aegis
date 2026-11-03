//! Sub-router for `POST /api/log-ingest/submit`.

use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;
use crate::transport::http::log_ingest::handlers;

/// Sub-router exposing `POST /api/log-ingest/submit`. Mounted at
/// `/api/log-ingest` from the top-level router so the path is
/// `POST /api/log-ingest/submit`.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(utoipa_axum::routes!(handlers::submit))
}

#[cfg(test)]
mod tests {
    use crate::state::{AppState, test_support};
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let cfg = logging_utils::LogIngestorConfig::builder()
            .log_dir(tempfile::tempdir().unwrap().path().to_path_buf())
            .file_name_prefix("router-test.log".into())
            .build();
        let log_ingestor = Arc::new(logging_utils::LogIngestor::new(cfg).unwrap());
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

    /// Review Focus #4: the new route appears in
    /// `/api-docs/openapi.json` with bearer security.
    #[tokio::test]
    async fn openapi_includes_log_ingest_submit_with_bearer_security() {
        let app = crate::transport::http::router::router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 256 * 1024)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let op = &doc["paths"]["/api/log-ingest/submit"]["post"];
        assert!(
            op.is_object(),
            "POST /api/log-ingest/submit must appear in openapi"
        );
        assert_eq!(
            op["security"][0]["BearerAuth"],
            serde_json::Value::Array(vec![]),
            "log_ingest_submit must require BearerAuth",
        );
    }
}
