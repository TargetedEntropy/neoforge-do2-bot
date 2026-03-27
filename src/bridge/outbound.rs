use anyhow::Result;
use tracing::{debug, info, warn};

use crate::state::SharedState;

/// Forward a chat message to OpenClaw via the CLI and return the AI's reply.
pub async fn forward_chat(
    shared: &SharedState,
    sender: &str,
    content: &str,
    _whisper: bool,
) -> Result<Option<String>> {
    let message = format!("{} said in Minecraft chat: {}", sender, content);

    info!(sender, content, "Forwarding to OpenClaw");

    // Shell out to `openclaw agent` which handles the full WebSocket RPC
    // protocol including waiting for the final response.
    let mut cmd = tokio::process::Command::new("openclaw");
    cmd.arg("agent")
        .arg("--agent").arg("main")
        .arg("--message").arg(&message)
        .arg("--json")
        .arg("--timeout").arg("120");

    debug!(cmd = ?cmd, "Running openclaw agent");

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(status = %output.status, stderr = %stderr, "openclaw agent failed");
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    debug!(raw_len = stdout.len(), "openclaw agent output");

    // Parse the JSON output to extract the reply text
    let parsed: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "Failed to parse openclaw output");
            debug!(stdout = %stdout, "Raw output");
            return Ok(None);
        }
    };

    // Extract: result.payloads[0].text
    let reply = parsed
        .get("result")
        .and_then(|r| r.get("payloads"))
        .and_then(|p| p.as_array())
        .and_then(|a| a.first())
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    debug!(reply = ?reply, "Extracted reply");

    Ok(reply)
}
