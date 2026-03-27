mod bridge;
mod commands;
mod config;
mod handler;
mod state;

use azalea::prelude::*;
use tracing::{error, info};

use config::{AuthMode, Config};
use state::BotState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::load();
    let address = config.mc_address();

    info!(
        address = %address,
        auth = %match &config.auth {
            AuthMode::Offline { username } => format!("offline ({})", username),
            AuthMode::Microsoft { email } => format!("microsoft ({})", email),
        },
        openclaw = %config.openclaw_url,
        http_port = config.http_listen_port,
        "Starting azalea-bot"
    );

    // Create account based on auth mode
    let account = match &config.auth {
        AuthMode::Offline { username } => {
            info!("Using offline auth");
            Account::offline(username)
        }
        AuthMode::Microsoft { email } => {
            info!(email, "Authenticating with Microsoft...");
            match Account::microsoft(email).await {
                Ok(acct) => {
                    info!(username = %acct.username, "Microsoft auth successful");
                    acct
                }
                Err(e) => {
                    error!(error = %e, "Microsoft auth failed");
                    std::process::exit(1);
                }
            }
        }
    };

    // Create the action channel (HTTP server -> event handler)
    let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel();
    handler::set_action_receiver(action_rx);

    // Build state with config and action sender
    let bot_state = BotState::new(config, action_tx);

    // Spawn HTTP server for inbound commands from OpenClaw
    let shared_for_http = bot_state.shared.clone();
    tokio::spawn(async move {
        bridge::inbound::run_server(shared_for_http).await;
    });

    // Connect to MC server (blocks forever)
    let _ = ClientBuilder::new()
        .set_handler(handler::handle)
        .set_state(bot_state)
        .start(account, address.as_str())
        .await;
}
