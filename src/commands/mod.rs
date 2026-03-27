use azalea::prelude::*;
use tracing::info;

/// Actions the bot can perform in-game.
/// New capabilities (teleport, inventory, etc.) are added as variants here.
pub enum BotAction {
    SendChat { message: String },
    // Future:
    // Teleport { x: f64, y: f64, z: f64 },
    // RunCommand { command: String },
    // LookAt { x: f64, y: f64, z: f64 },
}

/// Execute a single action using the bot client.
pub fn execute(bot: &Client, action: BotAction) {
    match action {
        BotAction::SendChat { message } => {
            // MC chat limit is 256 chars; split long messages
            for chunk in split_chat(&message, 250) {
                info!(msg = %chunk, "Sending chat");
                bot.chat(&chunk);
            }
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
