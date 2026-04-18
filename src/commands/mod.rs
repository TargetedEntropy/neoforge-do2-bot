use azalea::prelude::*;
use tracing::info;

use crate::movement::MovementMode;
use crate::state::SharedState;

/// Actions the bot can perform in-game.
/// New capabilities (teleport, inventory, etc.) are added as variants here.
pub enum BotAction {
    SendChat { message: String },
    StartMovement { mode: Option<MovementMode> },
    SetMovementEnabled { enabled: bool },
    SetMovementMode { mode: MovementMode },
    // Future:
    // Teleport { x: f64, y: f64, z: f64 },
    // RunCommand { command: String },
    // LookAt { x: f64, y: f64, z: f64 },
}

/// Execute a single action using the bot client.
pub fn execute(bot: &Client, shared: &SharedState, action: BotAction) {
    match action {
        BotAction::SendChat { message } => {
            // MC chat limit is 256 chars; split long messages
            for chunk in split_chat(&message, 250) {
                info!(msg = %chunk, "Sending chat");
                bot.chat(&chunk);
            }
        }
        BotAction::StartMovement { mode } => {
            let mut movement = shared.movement.lock();
            if let Some(mode) = mode {
                movement.set_mode(mode);
            }
            movement.set_enabled(true);
            info!("Autonomous movement started");
        }
        BotAction::SetMovementEnabled { enabled } => {
            let mut movement = shared.movement.lock();
            movement.set_enabled(enabled);
            info!(enabled, "Updated autonomous movement enabled state");
        }
        BotAction::SetMovementMode { mode } => {
            let mut movement = shared.movement.lock();
            movement.set_mode(mode);
            info!(mode = mode.as_str(), "Updated autonomous movement mode");
        }
    }
}

/// Split a message into chunks that fit within MC's chat limit.
fn split_chat(msg: &str, max_len: usize) -> Vec<String> {
    if msg.len() <= max_len {
        return vec![msg.to_string()];
    }
    msg.chars()
        .collect::<Vec<_>>()
        .chunks(max_len)
        .map(|c| c.iter().collect())
        .collect()
}
