//! MCP method dispatch: `initialize`, `tools/list`, `tools/call`.
//!
//! Translates a parsed JSON-RPC request into the MCP result for the BMS tool
//! surface. `tools/list` advertises the gated tool descriptors; `tools/call`
//! runs one through the same registry the embedded agent uses (see
//! [`super::run`]). Tool-level failures (bad args, denied writes) surface as an
//! MCP tool result with `isError: true` — a successful JSON-RPC response whose
//! body tells the calling agent the tool refused — never as a transport error,
//! so an external agent can react the same way the embedded loop does.

use std::sync::Arc;

use awaken_runtime_contract::contract::tool::{Tool, ToolResult, ToolStatus};
use serde_json::{json, Value};

use super::run::{run_tool_call, CallOutcome};
use crate::store::Store;

/// The MCP protocol revision this adapter implements.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Result of `initialize`: advertise server identity and the tools capability.
pub fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "rubix-bms", "version": env!("CARGO_PKG_VERSION") }
    })
}

/// Result of `tools/list`: each tool's MCP descriptor (name, description, input
/// schema). Built from the same gated registry a tool call dispatches into, so
/// the advertised surface and the callable surface never drift.
pub fn list_tools_result(tools: &[Arc<dyn Tool>]) -> Value {
    let listed: Vec<Value> = tools
        .iter()
        .map(|tool| {
            let d = tool.descriptor();
            json!({
                "name": d.name,
                "description": d.description,
                "inputSchema": d.parameters,
            })
        })
        .collect();
    json!({ "tools": listed })
}

/// `tools/call` params: which tool and its arguments.
pub struct CallParams {
    pub name: String,
    pub arguments: Value,
}

impl CallParams {
    /// Parse the JSON-RPC `params` of a `tools/call`. `arguments` defaults to an
    /// empty object so a no-arg tool can be called with `params` omitting it.
    pub fn parse(params: &Value) -> Result<Self, String> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or("tools/call requires a string `name`")?
            .to_string();
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        Ok(Self { name, arguments })
    }
}

/// Run a `tools/call` against `tools`, returning the MCP call result. An unknown
/// tool is an error result (not a transport error) so the caller sees which tool
/// was rejected. A suspended write returns a non-error result whose structured
/// content carries the held run id for the operator approval flow.
pub async fn call_tool_result(
    store: &Store,
    tools: &[Arc<dyn Tool>],
    params: CallParams,
    thread_id: &str,
) -> Value {
    let Some(tool) = tools.iter().find(|t| t.descriptor().name == params.name) else {
        return error_content(format!("unknown tool `{}`", params.name));
    };
    match run_tool_call(store, tool.as_ref(), params.arguments, thread_id).await {
        Ok(CallOutcome::Done(result)) => tool_result_content(&result),
        Ok(CallOutcome::Suspended { run_id, result }) => suspended_content(&run_id, &result),
        Err(message) => error_content(message),
    }
}

/// Map a finished [`ToolResult`] to an MCP tool result. A tool `Error` status
/// becomes `isError: true`; success carries the data payload as structured
/// content plus a text rendering for plain MCP clients.
fn tool_result_content(result: &ToolResult) -> Value {
    let is_error = result.status == ToolStatus::Error;
    let text = result
        .message
        .clone()
        .unwrap_or_else(|| result.data.to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": result.data,
        "isError": is_error,
    })
}

/// A suspended write's MCP result: not an error (the call was accepted), but it
/// did not commit — the body tells the external agent the write awaits operator
/// approval and surfaces the held run id.
fn suspended_content(run_id: &str, result: &ToolResult) -> Value {
    let text = result
        .message
        .clone()
        .unwrap_or_else(|| "write awaiting operator approval".to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": { "status": "awaiting_approval", "run_id": run_id },
        "isError": false,
    })
}

/// An error tool result (unknown tool, internal failure). MCP carries tool
/// errors inside a successful response with `isError: true`.
fn error_content(message: String) -> Value {
    json!({
        "content": [{ "type": "text", "text": message }],
        "isError": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_params_defaults_empty_arguments() {
        let p = CallParams::parse(&json!({"name": "read_point"})).unwrap();
        assert_eq!(p.name, "read_point");
        assert_eq!(p.arguments, json!({}));
    }

    #[test]
    fn call_params_requires_name() {
        assert!(CallParams::parse(&json!({"arguments": {}})).is_err());
    }

    #[test]
    fn error_result_sets_is_error() {
        let v = tool_result_content(&ToolResult::error("write_point", "denied"));
        assert_eq!(v["isError"], true);
        assert_eq!(v["content"][0]["text"], "denied");
    }

    #[test]
    fn success_result_carries_structured_content() {
        let v = tool_result_content(&ToolResult::success("read_point", json!({"cur": 21.5})));
        assert_eq!(v["isError"], false);
        assert_eq!(v["structuredContent"]["cur"], 21.5);
    }

    #[test]
    fn initialize_advertises_tools_capability() {
        let v = initialize_result();
        assert!(v["capabilities"]["tools"].is_object());
        assert_eq!(v["serverInfo"]["name"], "rubix-bms");
    }
}
