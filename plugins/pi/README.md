# OMNI for Pi

This extension connects [Pi](https://github.com/mariozechner/pi) lifecycle events to the local `omni` binary so OMNI can distill tool output and preserve session context.

> [!IMPORTANT]
> This package is only a Pi extension shim. Install OMNI separately and make sure the `omni` CLI is available on your `PATH`.

## Install from Git

The OMNI repository exposes this extension through the root `package.json`, so Pi can discover it from the git source:

```bash
pi -e git:https://github.com/fajarhide/omni
```

For local development from a clone, load the source file directly:

```bash
pi -e /absolute/path/to/omni/plugins/pi/index.ts
```

## Environment

The extension runs `omni` directly and inherits Pi's process environment. It sets `OMNI_AGENT_ID=pi` for every OMNI invocation.

If your workflow needs Infisical or another environment loader, launch Pi through that tool instead of hardcoding secrets in this extension:

```bash
infisical run --silent --projectId=<project-id> -- pi -e git:https://github.com/fajarhide/omni
```

## Behavior

- `session_start` calls `omni --session-start`.
- `before_agent_start` injects pending OMNI system prompt additions once.
- `session_before_compact` calls `omni --pre-compact`.
- `tool_result` calls `omni --post-hook`.
- `edit` and `write` tool results are skipped because OMNI passes mutation tools through unchanged.
- OMNI subprocess failures, timeouts, empty output, invalid JSON, and over-16MB payloads fail open so Pi can keep its original result.

## Slash Command Toggle

You can toggle OMNI distillation per session:

```text
/omni
/omni status
/omni on
/omni off
/omni refresh
/omni help
```

- `on`: enable OMNI forwarding for this session.
- `off`: disable OMNI forwarding for this session (raw tool output pass-through).
- `refresh`: re-check OMNI availability.

Environment defaults:

```bash
PI_OMNI_ENABLED=1
PI_OMNI_SHOW_STATUS=1
PI_OMNI_VERBOSE=0
```

## Optional Configuration

By default, the extension executes `omni` from `PATH`. If Pi provides extension configuration, set `omniPath` to use a custom binary path:

```json
{
  "omniPath": "/usr/local/bin/omni"
}
```

## Probe Verification

Use a wrapper script during development to inspect traffic between Pi and OMNI:

```bash
cat >/tmp/omni-probe <<'SH'
#!/usr/bin/env bash
set -eu
{
  echo "--- args: $*"
  echo "--- OMNI_AGENT_ID: ${OMNI_AGENT_ID:-}"
  cat
  echo
} >>/tmp/omni-pi-probe.log
exec omni "$@"
SH
chmod +x /tmp/omni-probe
```

Then configure Pi to use `/tmp/omni-probe` as `omniPath` and verify:

1. `session_start` sends `hookEventName: "SessionStart"` and `OMNI_AGENT_ID=pi`.
2. `before_agent_start` injects any pending system prompt addition once.
3. `tool_result` sends normalized OMNI tool names such as `Bash`, `Read`, and `Grep`.
4. `edit` and `write` tool results are not forwarded.
5. `session_before_compact` sends `hookEventName: "PreCompact"`.
6. Probe errors, sleeps longer than 10 seconds, empty output, and payloads over 16MB all fail open.
