# OMNI for Antigravity IDE

## Installation

In Antigravity IDE:

1. Open Plugin Manager
2. Search for "OMNI Signal Engine"
3. Click Install

Or manually:

```bash
antigravity plugin install ./integrations/antigravity
```

## Configuration

OMNI works automatically. No configuration needed if `omni` is in PATH.

Optional in `~/.antigravity/plugins/omni.json`:

```json
{
  "omniPath": "/usr/local/bin/omni",
  "aggressiveness": "balanced"
}
```

## How It Works

All terminal tool calls are automatically distilled by OMNI via MCP protocol.
The distilled output appears in your AI chat, saving 80-90% tokens.

Use `omni stats` to monitor savings.
