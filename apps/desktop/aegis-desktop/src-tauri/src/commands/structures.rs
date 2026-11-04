use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::ApiError;
use crate::http::structures::{
    self, AddNodeRequest, CreateStructureRequest, NodeViewResponse, StructureLevelResponse,
    StructureSummaryResponse, StructureViewResponse, UpdateNodeRequest, UpdateStructureRequest,
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
pub async fn create_structure(
    app_state: tauri::State<'_, SharedAppState>,
    body: CreateStructureRequest,
) -> Result<StructureViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    create_structure_impl(app_state.inner(), &req_ctx, body).await
}

pub async fn create_structure_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    body: CreateStructureRequest,
) -> Result<StructureViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "create_structure"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                structures::create_structure(app_state.http_client(), &body).await
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
pub async fn update_structure(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
    body: UpdateStructureRequest,
) -> Result<StructureViewResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    update_structure_impl(app_state.inner(), &req_ctx, id, body).await
}

pub async fn update_structure_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
    body: UpdateStructureRequest,
) -> Result<StructureViewResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "update_structure"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                structures::update_structure(app_state.http_client(), id, &body).await
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
pub async fn delete_structure(
    app_state: tauri::State<'_, SharedAppState>,
    id: i64,
) -> Result<(), ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    delete_structure_impl(app_state.inner(), &req_ctx, id).await
}

pub async fn delete_structure_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    id: i64,
) -> Result<(), ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "delete_structure"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                structures::delete_structure(app_state.http_client(), id).await
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
    //!   create_structure:      { body }
    //!   update_structure:      { id, body }
    //!   delete_structure:      { id }
    //!   get_structure:         { id }
    //!   add_structure_node:    { structureId, body }
    //!   update_structure_node: { structureId, nodeId, body }
    //!
    //! The `body` carries the node's `entryName` — its own name, never a
    //! path. The nesting comes from `parent`.
    //!   delete_structure_node: { structureId, nodeId }
    //!
    //! These tests pin the deserialization shape so a future rename cannot
    //! silently change a command's invoke keys.

    use super::*;
    use serde_json::json;

    /// The invoke payload keys each command expects.
    ///
    /// Tauri v2 derives them from the command's Rust parameter names:
    /// `tauri-macros` lower-camel-cases each parameter, and `tauri`'s
    /// resolver looks that key up in the payload with no whole-payload
    /// fallback. So a wrapper that sends the wrong key is rejected at
    /// runtime with "missing required key body", not at compile time.
    ///
    /// The list lives here rather than in a shared file because the
    /// TypeScript side keeps its own copy — see
    /// `src/test/features/templates/api.test.ts`.
    const EXPECTED_INVOKE_KEYS: &[(&str, &[&str])] = &[
        ("list_structures", &["level"]),
        ("get_structure", &["id"]),
        ("create_structure", &["body"]),
        ("update_structure", &["id", "body"]),
        ("delete_structure", &["id"]),
        ("add_structure_node", &["structureId", "body"]),
        ("update_structure_node", &["structureId", "nodeId", "body"]),
        ("delete_structure_node", &["structureId", "nodeId"]),
    ];

    /// This test extracts the parameter names from the command signatures
    /// in this very file and checks them against
    /// [`EXPECTED_INVOKE_KEYS`], so renaming a Rust parameter breaks here
    /// instead of at runtime.
    #[test]
    fn invoke_keys_match_the_documented_shape() {
        let source = include_str!("structures.rs");

        for (command, expected) in EXPECTED_INVOKE_KEYS {
            let marker = format!("pub async fn {command}(");
            let start = source
                .find(&marker)
                .unwrap_or_else(|| panic!("command {command} not found in this file"));
            let params = &source[start + marker.len()..];
            let params = &params[..matching_paren(params)];

            let keys: Vec<String> = split_params(params)
                .iter()
                .filter_map(|p| p.split_once(':').map(|(name, _)| name.trim()))
                // `app_state` is the injected Tauri state, not an argument.
                .filter(|name| *name != "app_state")
                .map(to_lower_camel)
                .collect();

            let expected_keys: Vec<String> = expected.iter().map(|k| k.to_string()).collect();

            assert_eq!(keys, expected_keys, "invoke keys for {command}");
        }
    }

    /// The index of the `)` that closes the parameter list.
    fn matching_paren(s: &str) -> usize {
        let mut depth = 0usize;
        for (i, ch) in s.char_indices() {
            match ch {
                '(' | '<' | '[' => depth += 1,
                ')' | '>' | ']' => {
                    if depth == 0 {
                        return i;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        panic!("no closing parenthesis");
    }

    /// Split a parameter list on the commas that separate parameters.
    fn split_params(s: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut depth = 0usize;
        let mut current = String::new();
        for ch in s.chars() {
            match ch {
                '(' | '<' | '[' => {
                    depth += 1;
                    current.push(ch);
                }
                ')' | '>' | ']' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    out.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(ch),
            }
        }
        if !current.trim().is_empty() {
            out.push(current.trim().to_string());
        }
        out
    }

    /// `structure_id` → `structureId`, the shape Tauri v2 looks up.
    fn to_lower_camel(snake: &str) -> String {
        let mut out = String::new();
        for (i, part) in snake.split('_').enumerate() {
            if i == 0 {
                out.push_str(part);
            } else if let Some(first) = part.chars().next() {
                out.extend(first.to_uppercase());
                out.push_str(&part[1..]);
            }
        }
        out
    }

    #[test]
    fn add_node_request_deserializes_a_camel_case_payload() {
        let raw = json!({
            "entryName": "raw",
            "description": "d",
            "directory": true,
            "parent": null,
            "reference": null,
            "migration": null
        });
        let body: AddNodeRequest = serde_json::from_value(raw).unwrap();
        assert_eq!(body.entry_name, "raw");
        assert!(body.directory);
        assert!(body.parent.is_none());
    }

    #[test]
    fn add_node_request_rejects_a_snake_case_payload() {
        let raw = json!({ "structure_id": 3, "entryName": "raw" });
        let result: Result<AddNodeRequest, _> = serde_json::from_value(raw);
        assert!(
            result.is_err(),
            "a snake_case payload must not parse against AddNodeRequest"
        );
    }
}
