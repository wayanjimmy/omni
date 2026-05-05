# CLAUDE.md — OMNI Developer Guide

This file is for AI assistants (Claude, Codex, etc.) and human contributors working on OMNI.

## Quick Reference

```bash
cargo build              # Build debug binary
cargo build --release    # Build release binary
cargo test               # Run all tests
cargo test <module>      # Run specific module tests
cargo insta review       # Review snapshot test changes
cargo clippy             # Lint
cargo fmt                # Format
```

## Project Structure

```
src/
├── main.rs              # CLI dispatch
├── lib.rs               # Library re-exports for integration tests
├── paths.rs             # Path resolution and discovery
├── agents/              # Multi-agent support (Claude, Cursor, Copilot, etc.)
├── cli/                 # Command-line interfaces
│   ├── init.rs, stats.rs, doctor.rs, learn.rs, session.rs
│   └── diff.rs, exec.rs, optimize.rs, reset.rs, rewind.rs, rewrite.rs, update.rs
├── distillers/          # Content filtering logic by type
│   ├── git.rs, build.rs, test.rs, cloud.rs, database.rs, jsts.rs
│   └── readfile.rs, search.rs, security.rs, system_ops.rs, vcs.rs
├── graph/               # Code graph indexing
│   └── indexer.rs
├── guard/               # Safety, security, limits, and trust bounds
│   ├── config.rs, env.rs, limits.rs, trust.rs, update.rs
├── hooks/               # Pipeline execution hooks
│   ├── dispatcher.rs    # Universal hook router
│   ├── pre_tool.rs, post_tool.rs, post_tool_failure.rs
│   └── session_start.rs, session_end.rs, pre_compact.rs, pipe.rs
├── mcp/                 # Model Context Protocol
│   └── server.rs        # MCP server tools
├── pipeline/            # Core processing pipeline
│   ├── analyzer.rs, collapse.rs, registry.rs, scorer.rs, toml_filter.rs
├── session/             # Session tracking and learning
│   ├── tracker.rs, learn.rs, correction.rs
├── store/               # Persistence layer
│   └── sqlite.rs, transcript.rs
└── util/                # Common utilities
    └── command_family.rs, token_estimate.rs
```

## Pipeline Architecture

```
Input (raw tool output)
  │
  ▼
┌─────────────────────────────────────────────┐
│ Stage 1: Registry                           │
│ registry::resolve_profile(command)          │
│ (Matches command to specialized distiller)  │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│ Stage 2: Scorer                             │
│ scorer::score_with_command(input, cmd, sess)│
│ → Vec<OutputSegment> with relevance scores  │
│ (Critical=1.0, Important=0.7, Noise=0.1)    │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│ Stage 3: Distiller                          │
│ distillers::distill_with_command(...)       │
│ → output_string                             │
│ Filters noise                               │
└─────────────┬───────────────────────────────┘
              │
              ▼
Output (distilled signal)
```

1. Create `src/distillers/my_type.rs`:
```rust
use crate::pipeline::{OutputSegment, SessionState};
use super::Distiller;

pub struct MyDistiller;

impl Distiller for MyDistiller {
    fn distill(
        &self, 
        segments: &[OutputSegment], 
        input: &str,
        session: Option<&SessionState>,
    ) -> String {
        // Extract and summarize the critical information
        todo!()
    }
}
```

2. Register in `src/distillers/mod.rs`:
```rust
pub mod my_type;
// In get_distiller():
ContentType::MyType => Box::new(my_type::MyDistiller),
```

3. Add a fixture file in `tests/fixtures/my_type_example.txt`

4. Add a snapshot test in `src/distillers/mod.rs`:
```rust
snapshot_test!(test_my_type_distillation, "my_type_example.txt", ContentType::MyType);
```

5. Run `cargo test` then `cargo insta review` to approve the snapshot.

## How to Add a TOML Filter

Create a file in `~/.omni/filters/my_filter.toml`:
```toml
schema_version = 1

[filters.my_filter]
description = "My custom filter"
match_command = "^my-tool\\b"
strip_lines_matching = ["^DEBUG", "^TRACE"]
max_lines = 50

[[tests.my_filter]]
name = "basic test"
input = "DEBUG: ignore\nIMPORTANT: keep"
expected = "IMPORTANT: keep"
```

Verify with: `omni learn --verify`

## Database Schema

- **sessions**: Session state (id, timestamps, task/domain hints, state JSON)
- **distillations**: Every distillation event (filter, input/output bytes, route, score, latency, agent_id)
- **file_access**: Hot file tracking per session
- **rewind_store**: Compressed content (SHA-256 hash → content, with retrieval counter)
- **session_events (FTS5)**: Full-text searchable event index
- **passthrough_events**: Telemetry for commands that bypass the pipeline
- **unhandled_tools**: Telemetry for tools that OMNI doesn't yet support natively
- **execution_traces**: Execution traces containing raw input and distilled output per command
- **session_summaries**: Summarized metrics per session
- **project_knowledge**: Cross-session semantic memory and knowledge
- **agent_sessions**: Shared state tracking across multiple agents

## Cross-Platform OS Best Practices

OMNI must compile and pass tests gracefully across Linux, macOS, and Windows. All agents and contributors strictly adhere to the following pillars:

1. **No Hardcoded Path Separators**: Never use `/` or `\\` directly for file paths. Always use `std::path::PathBuf` and `PathBuf::push()` to build paths dynamically.
2. **Line Endings (`\n` vs `\r\n`)**: Never hard-match exactly against `\n` in string assertions. Windows uses `\r\n`. Use `.lines()` iterator which gracefully cleans lines on all OS, or use `replace("\r\n", "\n")` before assertions.
3. **OS-Specific Executable Suffixes**: Never assume binary names are exactly `./omni`. In Windows it compiles as `omni.exe`. Use `std::env::consts::EXE_SUFFIX` for dynamic matching.
4. **Environment Variables**: Windows environment variables are case-insensitive. Unix is case-sensitive. Always use `eq_ignore_ascii_case()` when reading from `std::env::vars()` or interacting with os-injected configurations to prevent false-panics.
5. **Robust CI Matrix Testing**: Before declaring success, guarantee the feature respects the Github Actions CI matrix. Changes to system integration code must be fundamentally safe to execute against `windows-latest` alongside `ubuntu-latest`.

## Key Design Decisions

- **Panic safety**: All hooks use `catch_unwind` — OMNI never crashes the host agent
- **Graceful degradation**: If DB fails, hooks still work (just without session context)
- **Deterministic**: Same input always produces same output (no randomness)
- **Sub-millisecond**: Pipeline targets <2ms for typical inputs
- **Never drop**: RewindStore ensures no information is permanently lost

### Essential Reading
- [tests/README.md#critical-guardrails](tests/README.md#critical-guardrails) — **Mandatory: Prevent deadlocks & test hangs**
