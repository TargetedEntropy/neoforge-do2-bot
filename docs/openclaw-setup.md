# OpenClaw Agent Setup

This guide explains how to configure an OpenClaw agent to interact with the
Minecraft bot.

## Overview

The bot exposes an HTTP API at `http://localhost:3001` on the same machine where
it runs. OpenClaw sends commands to the bot via this API, and the bot forwards
player chat mentions to OpenClaw via webhooks.

```
Discord <-> OpenClaw <-> Bot HTTP API <-> Minecraft Server
                              |
                         localhost:3001
```

## OpenClaw Agent Files

Place these in your OpenClaw agent directory
(e.g., `~/.openclaw/agents/main/agent/`):

### TOOLS.md

Describes the bot's HTTP API so the agent knows how to send commands.

```markdown
# Tools

## Minecraft Bot (nf-do2-bot)

You have access to a Minecraft bot running on the DynamicOdyssey 2 modded server.

### Available Actions

#### Check if the bot is online

```bash
curl -s http://localhost:3001/health
```

Returns `ok` if the bot is connected.

#### Send a chat message in-game

```bash
curl -s -X POST http://localhost:3001/actions \
  -H "Content-Type: application/json" \
  -d '{"action":"chat","message":"Hello everyone!"}'
```

### Important Notes

- Messages limited to 250 characters (auto-split if longer)
- When players mention the bot's name in chat, the message is forwarded to you
- Respond by sending a chat message back through the API
- Do NOT start/stop/restart the bot — admin only
```

### SOUL.md

Defines the agent's personality and role.

```markdown
# Soul

You are an AI assistant with access to a Minecraft bot on the DynamicOdyssey 2
modded server.

When a player talks to the bot in Minecraft, their message is forwarded to you.
Read it, craft a friendly response, and send it back via the chat API.

Keep in-game responses under 200 characters. Players are gaming, not reading novels.
```

## Bot Configuration

The bot needs `OPENCLAW_URL` and optionally `OPENCLAW_TOKEN` set to forward
player chat mentions to OpenClaw.

In `azalea-bot.toml`:

```toml
[openclaw]
url = "http://127.0.0.1:18789"
token = "your-openclaw-bearer-token"
```

Or via environment variables:

```bash
OPENCLAW_URL=http://127.0.0.1:18789 OPENCLAW_TOKEN=secret ./target/release/azalea-bot
```

## How the Chat Bridge Works

### Player -> Bot -> OpenClaw -> Bot -> Player

1. Player types in Minecraft chat: `hey boostie whats up?`
2. Bot detects its name was mentioned
3. Bot POSTs to OpenClaw's `/hooks/agent`:
   ```json
   {"sender":"PlayerName","content":"hey boostie whats up?","source":"minecraft","whisper":false}
   ```
4. OpenClaw processes through the LLM using SOUL.md + TOOLS.md context
5. OpenClaw returns a reply
6. Bot sends the reply as Minecraft chat

### Discord -> OpenClaw -> Bot -> Minecraft

1. User sends message in Discord channel
2. OpenClaw agent decides to relay it to Minecraft
3. Agent executes:
   ```bash
   curl -s -X POST http://localhost:3001/actions \
     -H "Content-Type: application/json" \
     -d '{"action":"chat","message":"Message from Discord!"}'
   ```
4. Bot sends it in Minecraft chat

## API Reference

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

## Future Actions

The bot's action system is extensible. Future actions may include:

- `teleport` — move the bot to coordinates
- `command` — execute a server command
- `look` — look at a position
- `status` — report bot position, health, nearby players

These will be added as new variants in the bot's `BotAction` enum.
