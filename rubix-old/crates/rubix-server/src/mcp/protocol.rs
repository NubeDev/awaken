//! JSON-RPC 2.0 envelope types for the MCP HTTP transport.
//!
//! The outbound MCP adapter speaks the Model Context Protocol over a single
//! HTTP endpoint: each request is a JSON-RPC 2.0 call (`initialize`,
//! `tools/list`, `tools/call`) and each response a JSON-RPC result or error.
//! These types are the wire envelope only; the method dispatch and tool
//! semantics live in [`super::dispatch`].

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON-RPC 2.0 request. `id` is absent for notifications (which the adapter
/// acknowledges without a result body).
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    /// Must be `"2.0"`; other values are rejected as invalid requests.
    pub jsonrpc: String,
    /// Correlation id echoed in the response. Absent for notifications.
    #[serde(default)]
    pub id: Option<Value>,
    /// The method name (`initialize`, `tools/list`, `tools/call`).
    pub method: String,
    /// Method parameters; shape depends on the method.
    #[serde(default)]
    pub params: Value,
}

/// A JSON-RPC 2.0 response carrying exactly one of `result` or `error`.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// A successful result for request `id`.
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    /// An error response for request `id`.
    pub fn err(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// A JSON-RPC 2.0 error object. Codes follow the spec's reserved range for
/// protocol errors; tool-level failures surface inside a successful
/// `tools/call` result via the MCP `isError` flag, not here.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcError {
    /// `-32600`: the request was not a valid JSON-RPC 2.0 request.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
        }
    }

    /// `-32601`: the method is not supported by this server.
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("method not found: {method}"),
        }
    }

    /// `-32602`: the method's parameters were missing or malformed.
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn response_serializes_result_xor_error() {
        let ok = serde_json::to_value(JsonRpcResponse::ok(json!(1), json!({"a": 1}))).unwrap();
        assert_eq!(ok["jsonrpc"], "2.0");
        assert_eq!(ok["result"], json!({"a": 1}));
        assert!(ok.get("error").is_none());

        let err = serde_json::to_value(JsonRpcResponse::err(
            json!(2),
            JsonRpcError::method_not_found("foo/bar"),
        ))
        .unwrap();
        assert_eq!(err["error"]["code"], -32601);
        assert!(err.get("result").is_none());
    }

    #[test]
    fn request_defaults_missing_id_and_params() {
        let req: JsonRpcRequest =
            serde_json::from_value(json!({"jsonrpc": "2.0", "method": "tools/list"})).unwrap();
        assert!(req.id.is_none());
        assert_eq!(req.params, Value::Null);
    }
}
