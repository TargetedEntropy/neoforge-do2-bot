use std::sync::Arc;

use azalea::prelude::*;
use parking_lot::Mutex;
use tokio::sync::mpsc;

use crate::commands::BotAction;
use crate::config::{AuthMode, Config, MovementConfig};
use crate::movement::AutonomousMovement;

pub type ActionTx = mpsc::UnboundedSender<BotAction>;
pub type ActionRx = mpsc::UnboundedReceiver<BotAction>;

/// Azalea requires State to be Component + Clone + Default.
/// We store an Arc to shared mutable state so both the event handler
/// and the HTTP server can access it.
#[derive(Component, Clone, Default)]
pub struct BotState {
    pub shared: Arc<SharedState>,
}

/// Interior-mutable state shared between the MC event handler and HTTP server.
pub struct SharedState {
    pub config: Config,
    pub http_client: reqwest::Client,
    /// The HTTP server sends actions through this channel.
    /// Set to Some after the channel is created in main().
    pub action_sender: Mutex<Option<ActionTx>>,
    pub movement: Mutex<AutonomousMovement>,
}

impl Default for SharedState {
    fn default() -> Self {
        // This default is only needed to satisfy azalea's Component trait.
        // We always construct via BotState::new() with a real Config.
        Self {
            config: Config {
                mc_host: "localhost".into(),
                mc_port: 25566,
                auth: AuthMode::Offline {
                    username: "azalea_bot".into(),
                },
                openclaw_url: String::new(),
                openclaw_token: String::new(),
                http_listen_port: 3001,
                movement: MovementConfig {
                    enabled: false,
                    mode: crate::movement::MovementMode::Wander,
                    min_step_ticks: 8,
                    max_step_ticks: 20,
                    min_idle_ticks: 30,
                    max_idle_ticks: 80,
                    turn_degrees: 35.0,
                    unstuck_ticks: 30,
                    jump_cooldown_ticks: 80,
                },
            },
            http_client: reqwest::Client::new(),
            action_sender: Mutex::new(None),
            movement: Mutex::new(AutonomousMovement::new(MovementConfig {
                enabled: false,
                mode: crate::movement::MovementMode::Wander,
                min_step_ticks: 8,
                max_step_ticks: 20,
                min_idle_ticks: 30,
                max_idle_ticks: 80,
                turn_degrees: 35.0,
                unstuck_ticks: 30,
                jump_cooldown_ticks: 80,
            })),
        }
    }
}

impl BotState {
    pub fn new(config: Config, action_tx: ActionTx) -> Self {
        let movement_cfg = config.movement.clone();
        Self {
            shared: Arc::new(SharedState {
                config,
                http_client: reqwest::Client::new(),
                action_sender: Mutex::new(Some(action_tx)),
                movement: Mutex::new(AutonomousMovement::new(movement_cfg)),
            }),
        }
    }
}
