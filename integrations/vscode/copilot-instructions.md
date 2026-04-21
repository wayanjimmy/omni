# OMNI Signal Engine Instructions

When running terminal commands in this project, pipe output through OMNI
for distilled, AI-optimized output:

```bash
# Instead of:
cargo build

# Use:
cargo build 2>&1 | omni
# or:
omni exec cargo build
```

OMNI reduces token usage 80-90% by extracting only errors, warnings,
and actionable signals. Full output is stored in RewindStore and
retrievable via `omni_retrieve()` MCP tool.
