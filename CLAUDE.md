# nf-do2-bot — Minecraft Bot for Modded NeoForge Servers

## What This Is

A Rust bot using the Azalea library that connects to modded NeoForge 1.21.1 servers.
Bridges in-game chat to OpenClaw (AI agent gateway) for AI-powered responses via Discord.

### Key Specs

- **Minecraft**: 1.21.1 (protocol 767)
- **Server**: NeoForge 21.1.219
- **Bot**: Rust + Azalea (patched fork, nightly toolchain)
- **Auth**: Microsoft (device-code OAuth) or offline mode
- **AI Bridge**: OpenClaw via HTTP webhooks

---

## Repository Layout

```
nf-do2-bot/
├── CLAUDE.md              # This file
├── README.md              # User-facing docs
├── Cargo.toml             # Dependencies (patched azalea + simdnbt)
├── Cargo.lock
├── config.example.toml    # Example TOML config
├── src/
│   ├── main.rs            # Entry point: config, auth, HTTP server, MC connection
│   ├── config.rs          # CLI flags (clap), env vars, TOML config file
│   ├── state.rs           # BotState (Azalea Component) + SharedState (Arc)
│   ├── handler.rs         # Event handler: Login, Chat, Tick, Death, Disconnect
│   ├── bridge/
│   │   ├── mod.rs
│   │   ├── outbound.rs    # MC -> OpenClaw: POST /hooks/agent
│   │   ├── inbound.rs     # OpenClaw -> MC: axum HTTP server on port 3001
│   │   └── types.rs       # Shared JSON request/response types
│   └── commands/
│       └── mod.rs         # BotAction enum + execute() dispatcher
├── patches/               # Patched Rust crates
│   ├── azalea/            # Full 17-crate workspace
│   └── simdnbt-0.6.1/    # Patched NBT library
└── coremod/               # Server-side NeoForge coremod
    ├── azalea-bridge-2.5.0.jar   # Built coremod (drop into server mods/)
    ├── azalea_bridge.js          # ASM transformer source
    ├── build.sh                  # Rebuilds the JAR
    └── META-INF/                 # Coremod metadata
```

---

## Instructions for Agentic Bots

This section is for AI agents (OpenClaw, Claude Code, etc.) working on this project.

### What you CAN do

- Edit bot source code (anything under `src/`)
- Build the bot: `cargo build --release`
- Run the bot: `./target/release/azalea-bot [OPTIONS]`
- Send commands via HTTP API: `POST http://localhost:3001/actions`
- Check health: `GET http://localhost:3001/health`
- Edit the coremod: `coremod/azalea_bridge.js`
- Rebuild the coremod: `cd coremod && bash build.sh`

### What you MUST NOT do

- Do NOT start, stop, or manage any Minecraft server
- Do NOT modify server configuration files
- The server is managed by a human operator and assumed to be already running

---

## How to Build and Run

### Prerequisites

- Rust nightly: `rustup toolchain install nightly`
- The azalea-bridge coremod installed on the target server's `mods/` folder

### Build

```bash
cargo build --release
```

### Run

```bash
# Offline mode
./target/release/azalea-bot -s localhost -p 25566 -u azalea_bot

# Microsoft auth
./target/release/azalea-bot -s mc.example.com -p 25565 -e user@example.com

# With OpenClaw
OPENCLAW_URL=http://127.0.0.1:18789 OPENCLAW_TOKEN=secret \
  ./target/release/azalea-bot -s mc.example.com -e user@example.com
```

### CLI Flags and Environment Variables

```
-s, --server <HOST>       MC server hostname         [env: MC_HOST]
-p, --port <PORT>         MC server port             [env: MC_PORT]
-u, --username <NAME>     Offline mode username      [env: BOT_USERNAME]
-e, --email <EMAIL>       Microsoft auth email       [env: MS_EMAIL]
-c, --config <PATH>       TOML config file           [env: BOT_CONFIG]
    --openclaw-url <URL>  OpenClaw gateway URL       [env: OPENCLAW_URL]
    --openclaw-token <T>  OpenClaw bearer token      [env: OPENCLAW_TOKEN]
    --http-port <PORT>    Bot HTTP server port       [env: BOT_HTTP_PORT]
    RUST_LOG              Log level (e.g., azalea_bot=info)
```

Priority: **CLI flags > env vars > config file > defaults**

---

## Bot HTTP API

| Endpoint | Method | Body | Purpose |
|----------|--------|------|---------|
| `/health` | GET | — | Returns `"ok"` |
| `/actions` | POST | `{"action":"chat","message":"..."}` | Send chat in-game |

---

## Data Flow

**MC Chat -> OpenClaw -> MC Chat (player speaks to bot):**
1. Player mentions bot's name in chat
2. `Event::Chat` handler detects mention (case-insensitive via `bot.username()`)
3. Spawns async task to POST to OpenClaw `/hooks/agent`
4. OpenClaw returns AI reply
5. Bot sends reply as MC chat

**OpenClaw/Discord -> MC (external command):**
1. POST to `http://bot:3001/actions` with `{"action":"chat","message":"..."}`
2. axum handler queues a `BotAction` via mpsc channel
3. On next `Event::Tick` (20/sec), handler drains channel and executes

---

## Adding New Bot Capabilities

To add a new action (e.g., teleport):

1. Add variant to `BotAction` in `commands/mod.rs`
2. Add match arm in `commands::execute()`
3. Add match arm in `bridge::inbound::parse_action()`
4. Add fields to `InboundCommand` in `bridge/types.rs` if needed

---

## Patched Dependencies

Two crates in `patches/` with local modifications:

- **azalea** — `azalea-protocol/src/read.rs`: `MAXIMUM_UNCOMPRESSED_LENGTH` bumped
  from 2MB to 64MB (modded servers send ~25MB registry sync packets)
- **simdnbt** — patched for compatibility

Referenced via `path =` in Cargo.toml and `[patch.crates-io]` overrides.

---

## The Azalea Bridge Coremod

A single 4KB JAR (`coremod/azalea-bridge-2.5.0.jar`) that goes in the NeoForge
server's `mods/` folder. Uses ASM bytecode transformation at class-load time.
Real NeoForge players connect normally alongside the bot.

### What it patches

| Target | What it does |
|--------|-------------|
| `NetworkComponentNegotiator.negotiate()` | Preserves result for real clients; overrides failed negotiation for bot |
| `ConfigurationInitialization.configureModdedClient()` | Removes 3 validation tasks (RegistryDataMapNegotiation, CheckExtensibleEnums, CheckFeatureFlags) |
| `NetworkRegistry` (class-wide) | Neutralizes all `disconnect()` and `ATHROW` for unknown payloads |

### Rebuilding

```bash
cd coremod
vim azalea_bridge.js
bash build.sh
# Copy azalea-bridge-*.jar to server mods/ folder
```
