# OpenClaw Agent Setup

This guide explains how to configure an OpenClaw agent to interact with the
Minecraft bot.

## Overview

The bot runs on the same machine as OpenClaw. When a player mentions the bot
in Minecraft chat, the bot invokes `openclaw agent` as a subprocess to get an
AI response. OpenClaw can also send commands to the bot via its HTTP API.

```
Discord <-> OpenClaw <-> `openclaw agent` CLI <-> Bot <-> Minecraft Server
                              |
                    Bot HTTP API (localhost:3001)
```

## Prerequisites

- OpenClaw installed and running (`openclaw gateway`)
- The bot binary built and configured (`cargo build --release`)
- Both running on the same machine

## OpenClaw Agent Files

Place these in your OpenClaw agent directory
(e.g., `~/.openclaw/agents/main/agent/`):

### TOOLS.md

Teaches the agent how to interact with the Minecraft bot.

```markdown
# Tools

## Minecraft Bot

You have access to a Minecraft bot on a modded NeoForge server.

### Check if the bot is online

curl -s http://localhost:3001/health

Returns `ok` if the bot is connected to the Minecraft server.

### Send a chat message in-game

curl -s -X POST http://localhost:3001/actions \
  -H "Content-Type: application/json" \
  -d '{"action":"chat","message":"Hello everyone!"}'

### Important Notes

- Messages limited to 250 characters (auto-split if longer)
- When players mention the bot's name in chat, the message is forwarded to you
- Respond by sending a chat message back through the API
- Do NOT start/stop/restart the bot — admin only
```

### SOUL.md

Defines the agent's personality and behavior.

```markdown
# Soul

You are an AI assistant with access to a Minecraft bot on a modded server.

When a player talks to the bot in Minecraft, their message is forwarded to you.
Read it, craft a friendly response, and send it back via the chat API.

Keep in-game responses under 200 characters. Players are gaming, not reading novels.
```

## How the Chat Bridge Works

### Player -> Bot -> OpenClaw -> Bot -> Player

1. Player types in Minecraft chat: `hey BotName, what's up?`
2. Bot detects its name was mentioned (case-insensitive)
3. Bot runs: `openclaw agent --agent main --message "Player said: ..." --json`
4. OpenClaw processes through the LLM using SOUL.md + TOOLS.md context
5. Bot extracts `result.payloads[0].text` from the JSON output
6. Bot sends the reply as Minecraft chat

### Discord -> OpenClaw -> Bot -> Minecraft

1. User sends message in Discord channel bound to the OpenClaw agent
2. OpenClaw agent decides to relay it to Minecraft
3. Agent executes via its tools:
   ```bash
   curl -s -X POST http://localhost:3001/actions \
     -H "Content-Type: application/json" \
     -d '{"action":"chat","message":"Message from Discord!"}'
   ```
4. Bot sends it in Minecraft chat

## Bot HTTP API Reference

| Endpoint | Method | Body | Response |
|----------|--------|------|----------|
| `/health` | GET | — | `ok` (200) or connection refused |
| `/actions` | POST | `{"action":"chat","message":"..."}` | 200 OK or 400/503 |

### Error Codes

| Code | Meaning |
|------|---------|
| 200 | Action queued successfully |
| 400 | Bad request (unknown action or missing message) |
| 503 | Bot not connected to server yet |

## Bot Configuration

The `openclaw` CLI must be in the bot's `PATH`. No URL or token configuration
is needed since the bot invokes `openclaw agent` directly as a subprocess.

The bot config file (`azalea-bot.toml`) only needs server and auth settings:

```toml
[server]
host = "mc.example.com"
port = 25565

[auth]
mode = "microsoft"
email = "user@example.com"

[bot]
http_port = 3001
```

## Running with Debug Logging

To see exactly what the bot processes:

```bash
RUST_LOG=azalea_bot=debug ./target/release/azalea-bot
```

This logs every chat packet (raw message, parsed sender, content, mention
detection) and every OpenClaw interaction (CLI command, response parsing).

## Future Actions

The bot's action system is extensible. Future actions may include:

- `teleport` — move the bot to coordinates
- `command` — execute a server command
- `look` — look at a position
- `status` — report bot position, health, nearby players

These will be added as new variants in the bot's `BotAction` enum.
