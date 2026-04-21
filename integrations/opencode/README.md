# OMNI Signal Engine for OpenCode

## Installation

```bash
# Install OMNI
curl -fsSL https://raw.githubusercontent.com/fajarhide/omni/main/scripts/install.sh | sh

# Install plugin
opencode plugins install ./integrations/opencode
# or from registry:
opencode plugins install @omni/opencode-plugin
```

## Usage

OpenCode will automatically use `omni_shell` instead of the standard shell tool.

Monitor savings:

```bash
omni stats --today
```

## Token Savings

In a typical coding session: 80-90% reduction in shell output tokens.
