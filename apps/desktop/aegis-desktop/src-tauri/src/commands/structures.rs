use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::ApiError;
use crate::http::structures::{
    self, AddNodeRequest, NodeViewResponse, StructureLevelResponse, StructureSummaryResponse,
    StructureViewResponse, UpdateNodeRequest,
};
use crate::state::{Caller, RequestContext, SharedAppState};

// Tauri command argument conventions:
// Every other command in this crate (see commands/crf/form.rs,
// commands/user.rs, etc.) takes its arguments as separate function
// parameters — Tauri maps each JSON key in the args object to a
// parameter by camelCase name. The `level` string is re-parsed via
// serde's JSON deserialize so the call site gets a validation error
// back as `ApiError::Parse` instead of an opaque Tauri decode error.

fn parse_level(s: &str) -> Result<StructureLevelResponse, ApiError> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|e| ApiError::Parse {
        message: e.to_string(),
    })
}

#[tauri::command]
pub async fn list_structures(
    app_state: tauri::State<'_, SharedAppState>,
    level: Option<String>,
) -> Result<Vec<StructureSummaryResponse>, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    list_structures_impl(app_state.inner(), &req_ctx, level).await
}

pub async fn list_structures_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    level: Option<String>,
) -> Result<Vec<StructureSummaryResponse>, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "list_structures"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                let level = match level.as_deref() {
                    Some(l) => Some(parse_level(l)?),
                    None => None,
                };
                structures::list_structures(app_state.http_client(), level).await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn get_structure(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<StructureViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    get_structure_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn get_structure_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<StructureViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "get_structure"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                structures::get_structure(app_state.http_client(), id).await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn add_structure_node(
    app_state: tauri::State<'_, SharedAppState>,
    structure_id: i64,
    body: AddNodeRequest,
) -> Result<NodeViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    add_structure_node_impl(app_state.inner(), &req_ctx, structure_id, body).await
}

pub async fn add_structure_node_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    structure_id: i64,
    body: AddNodeRequest,
) -> Result<NodeViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "add_structure_node"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                structures::add_node(app_state.http_client(), structure_id, &body).await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn update_structure_node(
    app_state: tauri::State<'_, SharedAppState>,
    structure_id: i64,
    node_id: String,
    body: UpdateNodeRequest,
) -> Result<NodeViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_structure_node_impl(app_state.inner(), &req_ctx, structure_id, node_id, body).await
}

pub async fn update_structure_node_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    structure_id: i64,
    node_id: String,
    body: UpdateNodeRequest,
) -> Result<NodeViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_structure_node"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                structures::update_node(
                    app_state.http_client(),
                    structure_id,
                    &node_id,
                    &body,
                )
                .await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[tauri::command]
pub async fn delete_structure_node(
    app_state: tauri::State<'_, SharedAppState>,
    structure_id: i64,
    node_id: String,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_structure_node_impl(app_state.inner(), &req_ctx, structure_id, node_id).await
}

pub async fn delete_structure_node_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    structure_id: i64,
    node_id: String,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_structure_node"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                structures::delete_node(app_state.http_client(), structure_id, &node_id).await
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    //! Wire-shape tests. Tauri routes JSON keys to function parameters by
    //! camelCase name, so the frontend wrappers must send exactly:
    //!
    //!   list_structures:       { level? }
    //!   get_structure:         { id }
    //!   add_structure_node:    { structureId, body }
    //!   update_structure_node: { structureId, nodeId, body }
    //!   delete_structure_node: { structureId, nodeId }
    //!
    //! These tests pin the deserialization shape so a future rename cannot
    //! silently desync Rust and TS.

    use super::*;
    use serde_json::json;

    #[test]
    fn add_node_request_deserializes_a_camel_case_payload() {
        let raw = json!({
            "path": "raw",
            "description": "d",
            "directory": true,
            "parent": null,
            "reference": null,
            "migration": null
        });
        let body: AddNodeRequest = serde_json::from_value(raw).unwrap();
        assert_eq!(body.path, "raw");
        assert!(body.directory);
        assert!(body.parent.is_none());
    }

    #[test]
    fn add_node_request_rejects_a_snake_case_payload() {
        let raw = json!({ "structure_id": 3, "path": "raw" });
        let result: Result<AddNodeRequest, _> = serde_json::from_value(raw);
        assert!(
            result.is_err(),
            "a snake_case payload must not parse against AddNodeRequest"
        );
    }
}
