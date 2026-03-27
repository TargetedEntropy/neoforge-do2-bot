use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;
use tracing::info;

/// Azalea Bot — Minecraft bot with OpenClaw integration
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Server hostname
    #[arg(short = 's', long, env = "MC_HOST")]
    server: Option<String>,

    /// Server port
    #[arg(short = 'p', long, env = "MC_PORT")]
    port: Option<u16>,

    /// Offline mode username (skips Microsoft auth)
    #[arg(short = 'u', long, env = "BOT_USERNAME")]
    username: Option<String>,

    /// Microsoft account email (enables online auth)
    #[arg(short = 'e', long, env = "MS_EMAIL")]
    email: Option<String>,

    /// OpenClaw gateway URL
    #[arg(long, env = "OPENCLAW_URL")]
    openclaw_url: Option<String>,

    /// OpenClaw bearer token
    #[arg(long, env = "OPENCLAW_TOKEN")]
    openclaw_token: Option<String>,

    /// Bot HTTP server port (for inbound commands)
    #[arg(long, env = "BOT_HTTP_PORT")]
    http_port: Option<u16>,

    /// Path to config file
    #[arg(short = 'c', long, env = "BOT_CONFIG")]
    config: Option<PathBuf>,
}

/// TOML config file structure
#[derive(Deserialize, Default)]
struct FileConfig {
    #[serde(default)]
    server: ServerSection,
    #[serde(default)]
    auth: AuthSection,
    #[serde(default)]
    openclaw: OpenClawSection,
    #[serde(default)]
    bot: BotSection,
}

#[derive(Deserialize, Default)]
struct ServerSection {
    host: Option<String>,
    port: Option<u16>,
}

#[derive(Deserialize, Default)]
struct AuthSection {
    /// "microsoft" or "offline"
    mode: Option<String>,
    /// Microsoft account email
    email: Option<String>,
    /// Offline mode username
    username: Option<String>,
}

#[derive(Deserialize, Default)]
struct OpenClawSection {
    url: Option<String>,
    token: Option<String>,
}

#[derive(Deserialize, Default)]
struct BotSection {
    http_port: Option<u16>,
}

/// Auth mode for the bot
#[derive(Clone)]
pub enum AuthMode {
    Offline { username: String },
    Microsoft { email: String },
}

/// Resolved configuration (CLI > env > config file > defaults)
#[derive(Clone)]
pub struct Config {
    pub mc_host: String,
    pub mc_port: u16,
    pub auth: AuthMode,
    pub openclaw_url: String,
    pub openclaw_token: String,
    pub http_listen_port: u16,
}

impl Config {
    pub fn load() -> Self {
        let cli = Cli::parse();

        // Load config file if specified, or try default location
        let file_cfg = load_config_file(cli.config.as_deref());

        // Resolve: CLI > env (handled by clap) > config file > defaults
        let mc_host = cli.server
            .or(file_cfg.server.host)
            .unwrap_or_else(|| "localhost".into());

        let mc_port = cli.port
            .or(file_cfg.server.port)
            .unwrap_or(25566);

        let openclaw_url = cli.openclaw_url
            .or(file_cfg.openclaw.url)
            .unwrap_or_else(|| "http://127.0.0.1:18789".into());

        let openclaw_token = cli.openclaw_token
            .or(file_cfg.openclaw.token)
            .unwrap_or_default();

        let http_listen_port = cli.http_port
            .or(file_cfg.bot.http_port)
            .unwrap_or(3001);

        // Auth mode: --email flag takes priority, then config file, then --username/offline
        let auth = if let Some(email) = cli.email.or(file_cfg.auth.email.clone()) {
            AuthMode::Microsoft { email }
        } else if let Some(mode) = &file_cfg.auth.mode {
            if mode == "microsoft" {
                let email = file_cfg.auth.email.unwrap_or_default();
                if email.is_empty() {
                    panic!("Config has auth.mode = \"microsoft\" but no auth.email set");
                }
                AuthMode::Microsoft { email }
            } else {
                let username = cli.username
                    .or(file_cfg.auth.username)
                    .unwrap_or_else(|| "azalea_bot".into());
                AuthMode::Offline { username }
            }
        } else {
            let username = cli.username
                .or(file_cfg.auth.username)
                .unwrap_or_else(|| "azalea_bot".into());
            AuthMode::Offline { username }
        };

        Self { mc_host, mc_port, auth, openclaw_url, openclaw_token, http_listen_port }
    }

    pub fn mc_address(&self) -> String {
        format!("{}:{}", self.mc_host, self.mc_port)
    }

    pub fn display_username(&self) -> &str {
        match &self.auth {
            AuthMode::Offline { username } => username,
            AuthMode::Microsoft { email } => email,
        }
    }
}

fn load_config_file(explicit_path: Option<&std::path::Path>) -> FileConfig {
    let path = if let Some(p) = explicit_path {
        p.to_path_buf()
    } else {
        // Try default locations
        let candidates = [
            PathBuf::from("azalea-bot.toml"),
            PathBuf::from("config.toml"),
            dirs::config_dir()
                .map(|d| d.join("azalea-bot").join("config.toml"))
                .unwrap_or_default(),
        ];
        match candidates.iter().find(|p| p.exists()) {
            Some(p) => p.clone(),
            None => return FileConfig::default(),
        }
    };

    if !path.exists() {
        if explicit_path.is_some() {
            panic!("Config file not found: {}", path.display());
        }
        return FileConfig::default();
    }

    info!(path = %path.display(), "Loading config file");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));

    toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e))
}
