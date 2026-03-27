use serde::{Deserialize, Serialize};

/// Sent to OpenClaw when a player speaks to the bot in MC chat.
#[derive(Serialize)]
pub struct ChatForward {
    pub sender: String,
    pub content: String,
    pub source: &'static str,
    pub whisper: bool,
}

/// Response from OpenClaw's /hooks/agent endpoint.
#[derive(Deserialize)]
pub struct OpenClawResponse {
    /// The AI's reply text, if any.
    pub reply: Option<String>,
}

/// Inbound command from OpenClaw/Discord to the bot's HTTP server.
#[derive(Deserialize)]
pub struct InboundCommand {
    pub action: String,
    #[serde(default)]
    pub message: Option<String>,
    // Future fields for teleport, etc:
    // pub x: Option<f64>,
    // pub y: Option<f64>,
    // pub z: Option<f64>,
}
