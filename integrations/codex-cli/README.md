# OMNI for OpenAI Codex CLI

## Quick Install

```bash
bash integrations/codex-cli/install.sh
```

## Manual Setup

Add to `~/.codex/config.json`:

```json
{
  "shellWrapper": "~/.codex/omni-wrapper.sh"
}
```

## Verification

```bash
codex "run npm test and tell me if there are failures"
# Codex output will now be distilled by OMNI
omni stats  # check savings
```
