use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
use tracing::{debug, info, warn};

use crate::bridge::types::{RpcAgentParams, RpcRequest, RpcResponse};
use crate::state::SharedState;

/// Forward a chat message to OpenClaw via WebSocket JSON-RPC and return the AI's reply.
pub async fn forward_chat(
    shared: &SharedState,
    sender: &str,
    content: &str,
    _whisper: bool,
) -> Result<Option<String>> {
    let ws_url = http_to_ws(&shared.config.openclaw_url);
    let id = uuid::Uuid::new_v4().to_string();

    let message = format!("{} said in Minecraft chat: {}", sender, content);

    info!(sender, content, "Forwarding to OpenClaw via WebSocket RPC");

    // Build WebSocket request with auth header
    let mut request = ws_url.into_client_request()?;
    if !shared.config.openclaw_token.is_empty() {
        request.headers_mut().insert(
            "Authorization",
            format!("Bearer {}", shared.config.openclaw_token).parse()?,
        );
    }

    // Connect
    let (ws_stream, _) = tokio_tungstenite::connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();

    // Send JSON-RPC agent request
    let rpc = RpcRequest {
        jsonrpc: "2.0",
        id: id.clone(),
        method: "agent",
        params: RpcAgentParams {
            message,
            agent_id: "main".to_string(),
            idempotency_key: id.clone(),
        },
    };

    let rpc_json = serde_json::to_string(&rpc)?;
    write
        .send(Message::Text(rpc_json.into()))
        .await?;

    // Read messages until we get our response (matching id)
    let timeout = tokio::time::Duration::from_secs(120);
    let result = tokio::time::timeout(timeout, async {
        while let Some(msg) = read.next().await {
            let msg = msg?;
            match &msg {
                Message::Text(text) => {
                    debug!(raw_len = text.len(), "WS message received");

                    // Try parsing as our RPC response
                    match serde_json::from_str::<RpcResponse>(text.as_ref()) {
                        Ok(resp) => {
                            debug!(
                                resp_id = ?resp.id,
                                has_result = resp.result.is_some(),
                                has_error = resp.error.is_some(),
                                "Parsed RPC response"
                            );
                            if resp.id.as_deref() == Some(&id) {
                                if let Some(err) = resp.error {
                                    warn!(error = %err, "OpenClaw RPC error");
                                    return Ok(None);
                                }
                                if let Some(result) = resp.result {
                                    debug!(
                                        status = ?result.status,
                                        summary = ?result.summary,
                                        has_inner = result.result.is_some(),
                                        "RPC result details"
                                    );
                                    let reply = result
                                        .result
                                        .and_then(|r| {
                                            debug!(has_payloads = r.payloads.is_some(), "Inner result");
                                            r.payloads
                                        })
                                        .and_then(|p| {
                                            debug!(payload_count = p.len(), "Payloads");
                                            p.into_iter().next()
                                        })
                                        .and_then(|p| {
                                            debug!(text = ?p.text, "First payload");
                                            p.text
                                        });
                                    return Ok(reply);
                                }
                                return Ok(None);
                            } else {
                                debug!("Response id mismatch, waiting for ours");
                            }
                        }
                        Err(e) => {
                            // Log first 500 chars of unparseable messages
                            let preview: String = text.chars().take(500).collect();
                            debug!(error = %e, preview, "WS message not RPC response");
                        }
                    }
                }
                Message::Close(_) => {
                    debug!("WS connection closed by server");
                    break;
                }
                other => {
                    debug!(kind = ?other, "Non-text WS message");
                }
            }
        }
        Ok(None)
    })
    .await;

    // Close the connection
    let _ = write.send(Message::Close(None)).await;

    match result {
        Ok(reply) => reply,
        Err(_) => {
            warn!("OpenClaw RPC timed out after {}s", timeout.as_secs());
            Ok(None)
        }
    }
}

/// Convert http:// URL to ws:// for WebSocket connection.
fn http_to_ws(url: &str) -> String {
    if url.starts_with("https://") {
        url.replacen("https://", "wss://", 1)
    } else if url.starts_with("http://") {
        url.replacen("http://", "ws://", 1)
    } else if url.starts_with("ws://") || url.starts_with("wss://") {
        url.to_string()
    } else {
        format!("ws://{}", url)
    }
}
