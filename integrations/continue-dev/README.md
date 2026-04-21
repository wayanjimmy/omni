# OMNI Signal Engine for Continue.dev

Integrates OMNI's intelligent output distillation into [Continue.dev](https://continue.dev/) (VS Code extension).

## Prerequisites

- OMNI binary installed: `curl -fsSL https://raw.githubusercontent.com/fajarhide/omni/main/scripts/install.sh | sh`
- Continue.dev VS Code extension installed

## Installation

```bash
cd integrations/continue-dev
npm install && npm run build
```

Add to `~/.continue/config.json`:

```json
{
  "contextProviders": [
    {
      "name": "omni",
      "params": {}
    }
  ]
}
```

## Usage

In Continue.dev chat, type:

```
@omni run: npm test
```

OMNI will execute `npm test` and inject only the distilled output (errors, failures, summary) into your conversation context. Token savings are typically 80-90%.

## Custom OMNI Path

```json
{
  "contextProviders": [
    {
      "name": "omni",
      "params": {
        "omniPath": "/custom/path/to/omni"
      }
    }
  ]
}
```

## Token Savings

View savings with: `omni stats`
