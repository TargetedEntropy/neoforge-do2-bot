use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::{error, info, warn};

use crate::bridge::types::InboundCommand;
use crate::commands::BotAction;
use crate::state::SharedState;

/// Start the HTTP server that receives commands from OpenClaw/Discord.
pub async fn run_server(shared: Arc<SharedState>) {
    let port = shared.config.http_listen_port;

    let app = Router::new()
        .route("/actions", post(handle_action))
        .route("/health", get(handle_health))
        .with_state(shared);

    let addr = format!("0.0.0.0:{port}");
    info!(addr, "Bot HTTP server listening");

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(addr, error = %e, "Failed to bind HTTP server — inbound commands disabled");
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!(error = %e, "Bot HTTP server crashed");
    }
}

async fn handle_action(
    State(shared): State<Arc<SharedState>>,
    Json(cmd): Json<InboundCommand>,
) -> Result<StatusCode, StatusCode> {
    info!(action = %cmd.action, "Received inbound command");
    let action = parse_action(cmd)?;

    let sender = shared.action_sender.lock();
    match sender.as_ref() {
        Some(tx) => {
            tx.send(action).map_err(|_| {
                warn!("Action channel closed");
                StatusCode::SERVICE_UNAVAILABLE
            })?;
            info!("Action queued successfully");
            Ok(StatusCode::OK)
        }
        None => {
            warn!("Bot not connected yet, action_sender is None");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

async fn handle_health() -> &'static str {
    "ok"
}

fn parse_action(cmd: InboundCommand) -> Result<BotAction, StatusCode> {
    match cmd.action.as_str() {
        "chat" => {
            let message = cmd.message.ok_or(StatusCode::BAD_REQUEST)?;
            Ok(BotAction::SendChat { message })
        }
        // Future:
        // "teleport" => { ... }
        _ => {
            warn!(action = %cmd.action, "Unknown action");
            Err(StatusCode::BAD_REQUEST)
        }
    }
}
