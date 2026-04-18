use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request for OpenClaw gateway WebSocket.
#[derive(Serialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub id: String,
    pub method: &'static str,
    pub params: RpcAgentParams,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAgentParams {
    pub message: String,
    pub agent_id: String,
    pub idempotency_key: String,
}

/// JSON-RPC 2.0 response from OpenClaw gateway.
#[derive(Deserialize)]
pub struct RpcResponse {
    pub id: Option<String>,
    pub result: Option<RpcResult>,
    pub error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct RpcResult {
    pub status: Option<String>,
    pub summary: Option<String>,
    pub result: Option<RpcInnerResult>,
}

#[derive(Deserialize)]
pub struct RpcInnerResult {
    pub payloads: Option<Vec<RpcPayload>>,
}

#[derive(Deserialize)]
pub struct RpcPayload {
    pub text: Option<String>,
}

/// Inbound command from OpenClaw/Discord to the bot's HTTP server.
#[derive(Deserialize)]
pub struct InboundCommand {
    pub action: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    // Future fields for teleport, etc:
    // pub x: Option<f64>,
    // pub y: Option<f64>,
    // pub z: Option<f64>,
}
