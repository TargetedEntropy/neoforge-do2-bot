# nf-do2-bot

A Rust-based Minecraft bot that connects to heavily modded **NeoForge 1.21.1** servers
using the [Azalea](https://github.com/azalea-rs/azalea) library. Bridges in-game chat
to [OpenClaw](https://github.com/openclaw/openclaw) for AI-powered responses via Discord.

Built for the DynamicOdyssey modpack but works with any NeoForge 21.1.x server.

## Features

- Connects to modded NeoForge servers alongside real players
- Microsoft or offline authentication
- Listens for chat mentions, forwards to OpenClaw, relays AI responses
- HTTP API for sending commands into the game (chat + movement controls)
- Optional conservative autonomous movement (disabled by default)
- Stays connected persistently with proper Tick event processing
- CLI flags, env vars, and TOML config file support

## Prerequisites

- Rust nightly (`rustup toolchain install nightly`)
- The [azalea-bridge coremod](#server-coremod) installed on the target server

## Quick Start

```bash
# Build
cargo build --release

# Run (offline mode, local server)
./target/release/azalea-bot -s localhost -p 25566 -u azalea_bot

# Run (Microsoft auth, remote server)
./target/release/azalea-bot -s mc.example.com -p 25565 -e user@example.com
```

## Usage

```
Usage: azalea-bot [OPTIONS]

Options:
  -s, --server <HOST>       MC server hostname         [env: MC_HOST]      [default: localhost]
  -p, --port <PORT>         MC server port             [env: MC_PORT]      [default: 25566]
  -u, --username <NAME>     Offline mode username      [env: BOT_USERNAME] [default: azalea_bot]
  -e, --email <EMAIL>       Microsoft auth email       [env: MS_EMAIL]
  -c, --config <PATH>       Path to TOML config file   [env: BOT_CONFIG]
      --openclaw-url <URL>  OpenClaw gateway URL       [env: OPENCLAW_URL]
      --openclaw-token <T>  OpenClaw bearer token      [env: OPENCLAW_TOKEN]
      --http-port <PORT>    Bot HTTP server port       [env: BOT_HTTP_PORT] [default: 3001]
```

Config priority: **CLI flags > env vars > config file > defaults**

### Config File

Place an `azalea-bot.toml` or `config.toml` in the working directory, or at
`~/.config/azalea-bot/config.toml`. See [config.example.toml](config.example.toml).

```toml
[server]
host = "mc.example.com"
port = 25565

[auth]
mode = "microsoft"
email = "user@example.com"

[openclaw]
url = "http://127.0.0.1:18789"
token = "your-token"

[bot]
http_port = 3001

[movement]
# disabled by default for safety
enabled = false
mode = "wander"
min_step_ticks = 8
max_step_ticks = 20
min_idle_ticks = 30
max_idle_ticks = 80
turn_degrees = 35.0
unstuck_ticks = 30
jump_cooldown_ticks = 80
```

### Authentication

**Offline mode** (default): `-u azalea_bot`
- No Microsoft account needed
- Requires `online-mode=false` on the server

**Microsoft auth**: `-e user@example.com`
- Device-code OAuth flow (prints URL + code to visit)
- Tokens cached at `~/.minecraft/azalea-auth.json` (automatic refresh)
- Required for `online-mode=true` servers

## HTTP API

The bot runs an HTTP server (default port 3001) for receiving commands:

| Endpoint | Method | Body | Purpose |
|----------|--------|------|---------|
| `/health` | GET | — | Returns `"ok"` |
| `/actions` | POST | `{"action":"chat","message":"..."}` | Send a chat message |
| `/actions` | POST | `{"action":"movement_start"}` | Enable autonomous movement |
| `/actions` | POST | `{"action":"movement_start","mode":"wander"}` | Enable autonomous movement + mode |
| `/actions` | POST | `{"action":"movement_stop"}` | Disable autonomous movement |
| `/actions` | POST | `{"action":"movement_mode","mode":"wander"}` | Change movement mode |

```bash
# Send a chat message
curl -X POST http://localhost:3001/actions \
  -H "Content-Type: application/json" \
  -d '{"action":"chat","message":"Hello from Discord!"}'

# Start conservative autonomous movement
curl -X POST http://localhost:3001/actions \
  -H "Content-Type: application/json" \
  -d '{"action":"movement_start","mode":"wander"}'

# Stop movement
curl -X POST http://localhost:3001/actions \
  -H "Content-Type: application/json" \
  -d '{"action":"movement_stop"}'
```

## OpenClaw Integration

When a player mentions the bot's name in Minecraft chat:
1. Bot detects the mention (case-insensitive, with fallback parsing for modded chat formats)
2. Runs `openclaw agent --agent main --message "Player said: ..." --json` as a subprocess
3. OpenClaw processes through an LLM and returns a reply
4. Bot sends the reply as Minecraft chat

OpenClaw can also push commands TO the bot via the HTTP API above,
enabling Discord-to-Minecraft communication.

The `openclaw` CLI must be installed and accessible on the same machine as the bot.
See [docs/openclaw-setup.md](docs/openclaw-setup.md) for full setup instructions.

## Architecture

```
src/
├── main.rs          # Entry point: config, auth, HTTP server, MC connection
├── config.rs        # CLI flags (clap), env vars, TOML config file
├── state.rs         # BotState (Azalea Component) + SharedState (Arc)
├── handler.rs       # Event handler: Login, Chat, Tick, Death, Disconnect
├── movement.rs      # Conservative autonomous movement controller
├── bridge/
│   ├── mod.rs
│   ├── outbound.rs  # MC -> OpenClaw: via `openclaw agent` CLI
│   ├── inbound.rs   # OpenClaw -> MC: axum HTTP server
│   └── types.rs     # Shared JSON request/response types
└── commands/
    └── mod.rs       # BotAction enum + execute() dispatcher
```

### Adding New Capabilities

To add a new action (e.g., teleport):

1. Add a variant to `BotAction` in `commands/mod.rs`
2. Add a match arm in `commands::execute()`
3. Add a match arm in `bridge::inbound::parse_action()`
4. Add fields to `InboundCommand` in `bridge/types.rs` if needed

### Patched Dependencies

The bot uses local patched copies of two crates (in `patches/`):

- **azalea** — Full 17-crate workspace with `MAXIMUM_UNCOMPRESSED_LENGTH` bumped
  from 2MB to 64MB (heavily modded servers send ~25MB registry sync packets)
- **simdnbt** — NBT parsing library (patched for compatibility)

Requires **Rust nightly** (specified in `patches/azalea/rust-toolchain`).

## Server Coremod

The bot requires the **azalea-bridge** coremod installed on the NeoForge server.
This is a single 4KB JAR that goes in the server's `mods/` folder.

### Installation

```bash
cp coremod/azalea-bridge-2.5.0.jar /path/to/server/mods/
# Restart the server
```

Real NeoForge players can connect alongside the bot — the coremod is transparent
to normal clients.

### What it patches (via ASM bytecode transformation)

| Target | What it does |
|--------|-------------|
| `NetworkComponentNegotiator.negotiate()` | Preserves original result for real clients; overrides with success for the bot |
| `ConfigurationInitialization.configureModdedClient()` | Removes 3 validation tasks that fail for non-NeoForge clients |
| `NetworkRegistry` (class-wide) | Neutralizes `disconnect()` calls and `ATHROW` for unknown payloads |

### Rebuilding

```bash
cd coremod
# Edit azalea_bridge.js
bash build.sh
```

### Verifying

```bash
grep AzaleaBridge server/logs/latest.log
```

Expected:
```
[AzaleaBridge] v2.5.0 loaded
[AzaleaBridge] negotiate() patched: N return point(s)
[AzaleaBridge] Removed 3 problematic config task(s)
[AzaleaBridge] NetworkRegistry total: X disconnect(s), Y throw(s) neutralized
```

## Tested With

- **DynamicOdyssey 2.26.0** — NeoForge 21.1.219, 328 mods
- Bot connects, stays online, sends/receives chat, processes HTTP commands
- Real NeoForge players connect alongside the bot without issues

## License

MIT
