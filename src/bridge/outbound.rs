use anyhow::Result;
use tracing::{info, warn};

use crate::bridge::types::{ChatForward, OpenClawResponse};
use crate::state::SharedState;

/// Forward a chat message to OpenClaw and return the AI's reply.
pub async fn forward_chat(
    shared: &SharedState,
    sender: &str,
    content: &str,
    whisper: bool,
) -> Result<Option<String>> {
    let url = format!("{}/hooks/agent", shared.config.openclaw_url);

    let payload = ChatForward {
        sender: sender.to_string(),
        content: content.to_string(),
        source: "minecraft",
        whisper,
    };

    info!(sender, content, "Forwarding chat to OpenClaw");

    let mut req = shared.http_client.post(&url).json(&payload);

    if !shared.config.openclaw_token.is_empty() {
        req = req.bearer_auth(&shared.config.openclaw_token);
    }

    let resp = req.send().await?;

    if !resp.status().is_success() {
        warn!(status = %resp.status(), "OpenClaw returned non-success");
        return Ok(None);
    }

    let body: OpenClawResponse = resp.json().await?;
    Ok(body.reply)
}
