use std::sync::Arc;

use azalea::prelude::*;
use tracing::{info, warn};

use crate::bridge::outbound;
use crate::commands;
use crate::state::{ActionRx, BotState};

/// Wraps the action receiver for polling on Tick events.
/// Uses parking_lot::Mutex for non-async, non-blocking try_lock.
struct ActionDrain {
    rx: parking_lot::Mutex<ActionRx>,
}

static ACTION_RX: std::sync::OnceLock<ActionDrain> = std::sync::OnceLock::new();

/// Register the action receiver. Called once from main() before the bot starts.
pub fn set_action_receiver(rx: ActionRx) {
    let _ = ACTION_RX.set(ActionDrain {
        rx: parking_lot::Mutex::new(rx),
    });
}

pub async fn handle(bot: Client, event: Event, state: BotState) -> anyhow::Result<()> {
    let shared = &state.shared;

    match event {
        Event::Login => {
            info!("Bot logged in as {}", bot.username());
            bot.chat("azalea-bot online");
        }

        Event::Chat(packet) => {
            let (sender, content) = packet.split_sender_and_content();
            let sender = match sender {
                Some(s) => s,
                None => return Ok(()), // system message, skip
            };

            // Don't respond to our own messages
            let my_name = bot.username();
            if sender.eq_ignore_ascii_case(&my_name) {
                return Ok(());
            }

            info!(sender = %sender, content = %content, "Chat received");

            // Check if the bot was mentioned by name
            if !content.to_lowercase().contains(&my_name.to_lowercase()) {
                return Ok(());
            }

            info!("Bot was mentioned, forwarding to OpenClaw");

            // Forward to OpenClaw and relay the response
            let shared_clone = Arc::clone(shared);
            let bot_clone = bot.clone();
            let whisper = packet.is_whisper();

            tokio::spawn(async move {
                match outbound::forward_chat(&shared_clone, &sender, &content, whisper).await {
                    Ok(Some(reply)) => {
                        commands::execute(&bot_clone, commands::BotAction::SendChat {
                            message: reply,
                        });
                    }
                    Ok(None) => {
                        info!("OpenClaw returned no reply");
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to forward to OpenClaw");
                    }
                }
            });
        }

        Event::Tick => {
            // Drain any inbound actions from the HTTP server (non-blocking)
            if let Some(drain) = ACTION_RX.get() {
                if let Some(mut rx) = drain.rx.try_lock() {
                    while let Ok(action) = rx.try_recv() {
                        info!("Executing queued action");
                        commands::execute(&bot, action);
                    }
                }
            }
        }

        Event::Death(_) => {
            info!("Bot died, will auto-respawn");
        }

        Event::Disconnect(reason) => {
            let msg = reason
                .map(|r| r.to_string())
                .unwrap_or_else(|| "unknown".into());
            warn!(reason = %msg, "Disconnected from server");
        }

        _ => {}
    }

    Ok(())
}
