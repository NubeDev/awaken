//! Outbound MCP adapter — exposes the BMS tools to external agents.
//!
//! STACK-DEISGN.md "AI layer: awaken — Outbound: awaken's MCP/A2A/AG-UI adapters
//! expose the building to external agents and clients with the same gating."
//! This is the MCP surface: a single HTTP endpoint speaking JSON-RPC 2.0
//! (`initialize`, `tools/list`, `tools/call`). Tool calls dispatch into the same
//! [`build_tools_scoped`] registry the embedded agent uses, so an external agent
//! gets identical priority-array gating, tenant scoping (WS-07), and HITL
//! escalation (a held write lands in the `runs` registry, origin `mcp`).
//!
//! Auth is the platform's bearer middleware (WS-06): the request's [`Principal`]
//! both authorizes the call and, when it is site-pinned, confines the tools to
//! its `{org}/{site}`. An unauthenticated edge request runs with the full,
//! unscoped tool set, matching the chat surface.

mod dispatch;
mod protocol;
mod run;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use rubix_tools::TenantScope;
use serde_json::Value;

use crate::auth::RequestPrincipal;
use crate::tools::build_tools_scoped;
use crate::AppState;
use dispatch::{call_tool_result, initialize_result, list_tools_result, CallParams};
use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

/// Mount the MCP JSON-RPC endpoint.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/mcp", post(handle))
}

/// The tenant an external MCP session is confined to: the principal's site when
/// it is pinned to one, else `None` (a broader principal already passed auth for
/// its whole scope; an edge request has no principal). Mirrors the chat surface.
fn principal_scope(principal: &RequestPrincipal) -> Option<TenantScope> {
    let p = principal.0.as_ref()?;
    let org = p.scope.org.as_deref()?;
    let site = p.scope.site.as_deref()?;
    Some(TenantScope::new(org, site))
}

/// Handle one JSON-RPC request. The MCP HTTP transport posts a single JSON-RPC
/// object; this returns one response object (or `204` for a notification, which
/// carries no `id`). Method errors are JSON-RPC errors; tool errors ride inside
/// a successful `tools/call` result (see [`dispatch`]).
async fn handle(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<JsonRpcRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    if req.jsonrpc != "2.0" {
        return Json(JsonRpcResponse::err(
            req.id.unwrap_or(Value::Null),
            JsonRpcError::invalid_request("jsonrpc must be \"2.0\""),
        ))
        .into_response();
    }

    // A notification (no id) gets no result body, per JSON-RPC. MCP clients send
    // `notifications/initialized` after the handshake; acknowledge it silently.
    let Some(id) = req.id.clone() else {
        return axum::http::StatusCode::NO_CONTENT.into_response();
    };

    let response = match req.method.as_str() {
        "initialize" => JsonRpcResponse::ok(id, initialize_result()),
        "tools/list" => {
            let tools = build_tools_scoped(&state, principal_scope(&principal));
            JsonRpcResponse::ok(id, list_tools_result(&tools))
        }
        "tools/call" => match CallParams::parse(&req.params) {
            Ok(params) => {
                let scope = principal_scope(&principal);
                let tools = build_tools_scoped(&state, scope);
                // Key the external session by tool name + a session marker so a
                // suspended write's run is attributable; the MCP client supplies
                // no thread, so the call id stands in.
                let result = call_tool_result(&state.store, &tools, params, "mcp").await;
                JsonRpcResponse::ok(id, result)
            }
            Err(message) => JsonRpcResponse::err(id, JsonRpcError::invalid_params(message)),
        },
        other => JsonRpcResponse::err(id, JsonRpcError::method_not_found(other)),
    };
    Json(response).into_response()
}
