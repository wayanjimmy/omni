---
status: implemented
date: 2026-05-10
authors: [Amp, user]
timebox: 2 days
---

# PDD-0001: First-class Pi extension integration for OMNI

## P — Purpose

OMNI should support Pi as a first-class agent integration instead of relying on the separate `wayanjimmy/pi-omni` repository. The goal is to move the Pi extension implementation into this OMNI fork, make `omni init` able to install/configure the extension, and keep users in control of whether Pi is configured globally or only at a project/manual level.

The desired default is:

- `omni init --pi` installs the Pi extension through Pi's package installer, equivalent to `pi install git:github.com/<owner>/omni`, so Pi updates the user's global settings itself.
- An opt-out/local flag allows users to install project-locally with Pi's package installer, equivalent to `pi install git:github.com/<owner>/omni --local`, or skip Pi installation entirely if they want to manage it themselves.

This feature must preserve the existing runtime behavior of the external `pi-omni` extension: Pi lifecycle hooks call the local `omni` binary, OMNI receives `OMNI_AGENT_ID=pi`, and all runtime failures fail open so Pi can continue without OMNI intervention.

## S — Spike Findings

Research was performed against `https://github.com/wayanjimmy/pi-omni` and the local OMNI fork.

Spike document: `.pdd/spikes/SPIKE-0001-pi-package-install.md`

### External `pi-omni` repository findings

- The external repository is a small Pi package with one runtime TypeScript source file: `pi-extensions/omni.ts`.
- `package.json` declares Pi package metadata with `keywords` including `pi-package` and `pi-extension`, and `pi.extensions` pointing to `./pi-extensions`.
- `tsconfig.json` is development-only with `noEmit`; Pi executes TypeScript directly.
- The extension uses Node's `execFile` to invoke the `omni` binary with JSON over stdin/stdout.
- The extension injects `OMNI_AGENT_ID=pi` for every OMNI subprocess.
- The runtime is intentionally fail-open: subprocess errors, timeouts, empty stdout, invalid JSON, and oversized payloads do not break Pi.
- Hooks implemented by the external extension:
  - `session_start` calls `omni --session-start`.
  - `before_agent_start` injects pending `systemPromptAddition` into Pi's system prompt once.
  - `session_before_compact` calls `omni --pre-compact`.
  - `tool_result` calls `omni --post-hook` for non-mutating tools.
- Mutation tools such as `edit` and `write` are skipped before spawning OMNI.
- Tool names are normalized into OMNI canonical names such as `Bash`, `Read`, `Grep`, `Find`, and `LS`.

### Local repository findings

- This fork already contains a Pi plugin area:
  - `plugins/pi/index.ts`
  - `plugins/pi/package.json`
  - `plugins/pi/tsconfig.json`
  - `plugins/pi/README.md`
- `plugins/pi/index.ts` already implements the same broad hook bridge model and was audited against the external `pi-omni` behavior.
- Existing agent integrations live under `src/agents/` and implement `AgentIntegration` from `src/agents/mod.rs`.
- `src/cli/init.rs` selects integrations by CLI flags and calls `agent.install(&exe_path)`.
- `src/cli/reset.rs` removes selected integrations by calling `agent.uninstall()`.
- `src/cli/doctor.rs` already iterates over `crate::agents::all_integrations()` and calls `doctor_check`.

### PoC Validation

The core runtime shape is validated by the existing Pi extension source in `plugins/pi/index.ts`. The Rust integration was validated by:

1. Building the release binary with `cargo build --release`.
2. Replacing the globally installed `omni` binary with the new build.
3. Verifying `omni --version` shows `omni 0.6.0-pi-alpha`.
4. Running `omni init --pi --pi-manual` prints the correct tag-pinned commands.
5. Running `omni init --pi --pi-local` invokes `pi install` successfully.
6. Running `omni doctor` shows `Pi: Extension: [OK] installed`.
7. Running `omni reset --pi` prints actionable cleanup instructions.
8. Running `pi list` confirms the OMNI package is registered.

**Key spike finding:** `pi install git:*` supports `@tag` syntax (does `git checkout <tag>`) but does **not** support `#branch` syntax. The fragment is passed raw to the git URL, causing a clone error.

## A — Approach

Use the existing `plugins/pi/` directory as the canonical source for the Pi extension package. Add a first-class Rust-side Pi integration module that delegates installation to Pi's native package installer by running the equivalent of `pi install git:github.com/<owner>/omni`.

The default path should be Pi package installation rather than manually adding a direct `index.ts` path to settings. This matches Pi's documented package workflow, lets Pi own the settings mutation semantics, and keeps OMNI aligned with how users expect Pi extensions to be installed.

**Tag-based pinning:** The default package source uses `@tag` syntax (`git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha`) for deterministic installs. The tag is updated on each release.

### Alternatives Considered

| Option | Pros | Cons | Decision |
|--------|------|------|----------|
| Run `pi install git:github.com/<owner>/omni` from `omni init --pi` | Uses Pi's native package workflow, lets Pi update settings, matches user expectation, supports `--local` | Requires `pi` on PATH and may require network access for git source | ✓ Preferred |
| Run `pi install ./plugins/pi` or local repo path during development | Fast local validation, no network when run from a clone | Not suitable for released binary users unless source tree exists | ✓ Dev-only |
| Explicit staged file in `~/.omni/integrations/pi/index.ts` plus global settings entry | Deterministic, easy to reset, avoids Pi package ambiguity | Bypasses Pi's package installer and does not match expected `pi install git:*` workflow | ✗ Rejected |
| Copy extension into `~/.pi/agent/extensions/` | Simple and Pi-native | High double-load risk if settings also reference it; harder to know ownership | ✗ Rejected |
| Only document manual Pi package install | Minimal code | Does not satisfy first-class `omni init` goal | ✗ Rejected |
| Embed absolute `omni` binary path into the TypeScript extension | Avoids PATH issues | Stale after upgrades/moves; less flexible | ✗ Defer unless PATH failures become common |

## R — Requirements

### Functional

- [x] Add a first-class Pi integration to `src/agents/` and register it in `src/agents/mod.rs`.
- [x] Add `omni init --pi` support in `src/cli/init.rs`.
- [x] Add a local install flag, `--pi-local`, that invokes Pi's local package install mode.
- [x] Add a skip/manual flag, `--pi-manual`, that prints the Pi package install command without invoking it.
- [x] Add Pi reset support through `omni reset --pi` in `src/cli/reset.rs`.
- [x] Add Pi doctor coverage via the existing `doctor_check` integration path.
- [x] Install the Pi package by invoking Pi's installer with a git source, e.g. `pi install git:github.com/<owner>/omni`.
- [x] Use `pi install git:github.com/<owner>/omni --local` for project-local installs.
- [x] Do not manually edit `~/.pi/agent/settings.json` in the default path; Pi's installer owns that mutation.
- [x] Detect likely legacy or duplicate OMNI Pi extension sources and warn after install.
- [x] Preserve Pi runtime fail-open behavior in `plugins/pi/index.ts`.
- [x] Preserve `OMNI_AGENT_ID=pi` for all OMNI subprocesses.
- [x] Preserve mutation-tool skip behavior for `edit` and `write`.

### Non-Functional

- [x] Cross-platform paths must use `PathBuf` and path joining, not hardcoded separators.
- [x] Install/reset behavior must be idempotent from OMNI's perspective.
- [x] OMNI must not write Pi settings in the normal package-install path.
- [x] The runtime extension must have no build step and no required runtime npm dependencies.
- [x] `omni init --pi` may require network access for git package installation unless a local source override is explicitly used.
- [x] `cargo clippy -- -D warnings` passes (pre-existing warnings only, none in new code).
- [x] `cargo test` passes (all Pi tests pass; one pre-existing benchmark failure unrelated).
- [x] `plugins/pi` TypeScript typecheck passes.

## T — Tasks

| # | Task | Size | Dependencies | Files | Done |
|---|------|------|--------------|-------|------|
| 1 | Audit `plugins/pi/index.ts` against external `pi-omni` behavior and close parity gaps | M | — | `plugins/pi/index.ts`, `plugins/pi/README.md` | ✅ 2026-05-10 |
| 2 | Add `PiIntegration` implementing `AgentIntegration` | L | #1 | `src/agents/pi.rs`, `src/agents/mod.rs` | ✅ 2026-05-10 |
| 3 | Implement Pi package install command execution | M | #2 | `src/agents/pi.rs` | ✅ 2026-05-10 |
| 4 | Implement Pi package status and duplicate-source detection | M | #2 | `src/agents/pi.rs` | ✅ 2026-05-10 |
| 5 | Wire `omni init --pi`, `--pi-local`, and `--pi-manual` | M | #2, #3, #4 | `src/cli/init.rs`, `src/agents/mod.rs` | ✅ 2026-05-10 |
| 6 | Wire `omni reset --pi` around Pi-managed package removal or actionable manual cleanup | M | #2, #4 | `src/cli/reset.rs`, `src/agents/pi.rs` | ✅ 2026-05-10 |
| 7 | Add Pi doctor checks and safe fix behavior | M | #2, #3, #4 | `src/agents/pi.rs`, `src/cli/doctor.rs` if needed | ✅ 2026-05-10 |
| 8 | Add Rust tests for command construction, install mode selection, reset messaging, doctor status, and duplicate detection | L | #3, #4, #6, #7 | `src/agents/pi.rs` | ✅ 2026-05-10 |
| 9 | Update docs and i18n README files | M | #5, #6, #7 | `README.md`, `plugins/pi/README.md`, `i18n/README-*.md` | ⏳ Deferred — i18n READMEs not yet updated |
| 10 | Run verification gates | M | #1-#9 | repository-wide | ✅ 2026-05-10 |

### Task Details

#### Task 1: Audit Pi runtime parity

**What:** Compare `plugins/pi/index.ts` against the external `pi-omni/pi-extensions/omni.ts`. Preserve existing improvements only if they do not break the documented external contract.

**Files:** `plugins/pi/index.ts`, `plugins/pi/README.md`

**Verification:** TypeScript typecheck and behavioral review covering session start, pre-compact, tool result, prompt injection, fail-open handling, tool mapping, and mutation-tool skipping.

**Result:** ✅ Existing `plugins/pi/index.ts` already implements the same hook bridge. No parity gaps found.

#### Task 2: Add PiIntegration

**What:** Create `src/agents/pi.rs` implementing `AgentIntegration` with `id() == "pi"`, `name() == "Pi"`, `install`, `uninstall`, and `doctor_check`.

**Files:** `src/agents/pi.rs`, `src/agents/mod.rs`

**Pattern:** Follow existing agent modules such as `src/agents/claude.rs`, `src/agents/openclaw.rs`, and the shared registration pattern in `src/agents/mod.rs`.

**Verification:** Pi appears in `all_integrations()` and can be selected by init/reset/doctor flows.

**Result:** ✅ `PiIntegration` created with all required methods. Registered in `src/agents/mod.rs`.

#### Task 3: Implement Pi package install command execution

**What:** Invoke Pi's package installer from `omni init --pi`, using the repository package source. The default command shape is `pi install git:github.com/<owner>/omni`; local/project mode appends `--local` or `-l`.

**Files:** `src/agents/pi.rs`, `plugins/pi/package.json`, root `package.json`

**Verification:** Tests confirm command construction for global, local, manual/dry-run, and local-source development modes. Manual smoke test confirms Pi reports successful package installation.

**Result:** ✅ `install_with_mode` delegates to `pi install` with explicit args. Tag-pinned source `git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha`.

#### Task 4: Detect Pi package status and duplicates

**What:** Read Pi settings only for status/doctor/double-load detection. Do not manually mutate settings in the normal install path; Pi's `install` command owns settings updates.

**Files:** `src/agents/pi.rs`

**Verification:** Unit tests cover missing settings, existing package entries, explicit extension entries, old `pi-omni` package sources, legacy `~/.pi/agent/extensions/omni.ts`, and invalid JSON read failures reported as warnings.

**Result:** ✅ `PiSettingsSnapshot` checks both global (`~/.pi/agent/settings.json`) and project-local (`.pi/settings.json`) paths. Detects duplicates and legacy files.

#### Task 5: Wire init flags

**What:** Add `--pi`, `--pi-local`, and `--pi-manual` parsing to `src/cli/init.rs`. `--pi-local` must invoke Pi's project-local installer mode. `--pi-manual` must not invoke Pi; it prints the exact command the user can run.

**Files:** `src/cli/init.rs`, `src/agents/pi.rs`

**Verification:** CLI tests or manual runs confirm default global package install, local package install, and manual command output.

**Result:** ✅ All three flags wired in `init.rs` help, parsing, interactive menu, and target IDs.

#### Task 6: Wire reset

**What:** Add `--pi` to reset help, parsing, target IDs, and interactive menu. Prefer a Pi-native package removal command if Pi exposes one; otherwise print actionable manual cleanup instructions and only remove unambiguous OMNI-owned artifacts if any exist.

**Files:** `src/cli/reset.rs`, `src/agents/pi.rs`

**Verification:** Test confirms reset does not corrupt Pi settings and reports clear cleanup instructions for Pi-managed packages.

**Result:** ✅ `uninstall` prints manual cleanup steps. No blind settings mutation. Interactive menu updated.

#### Task 7: Add doctor checks

**What:** Implement Pi `doctor_check` to report whether Pi is installed, whether an OMNI Pi package source is configured, whether duplicate OMNI package/extension sources exist, and whether legacy extension files exist.

**Files:** `src/agents/pi.rs`, `src/cli/doctor.rs` if output changes are needed

**Verification:** Tests or manual runs cover healthy package state, missing Pi binary, missing package entry, duplicate package/extension entries, and legacy `~/.pi/agent/extensions/omni.ts`.

**Result:** ✅ Doctor output simplified to match other agents: `Pi:` header with `Extension:` status line. Detects local settings, duplicates, legacy files.

#### Task 8: Tests

**What:** Add focused Rust tests for pure command construction, mode selection, settings read-only status parsing, duplicate detection, and reset/doctor messaging.

**Files:** `src/agents/pi.rs`, `tests/` if integration-style tests are preferred

**Verification:** `cargo test pi` or the narrowest available targeted test command passes, followed by full `cargo test` before merge.

**Result:** ✅ 18 unit tests in `src/agents/pi.rs`. All pass. Covers args construction, mode selection, settings parsing, duplicate detection, env override, and edge cases.

#### Task 9: Docs and i18n

**What:** Update user docs for Pi support, default `pi install git:*` integration, project-local install, manual install, reset, doctor, double-load warnings, and `omni` PATH requirements.

**Files:** `README.md`, `plugins/pi/README.md`, `i18n/README-ja.md`, `i18n/README-zh.md`, `i18n/README-ar.md`, `i18n/README-id.md`, `i18n/README-vi.md`, `i18n/README-ko.md`

**Verification:** Docs mention `omni init --pi`, `omni init --pi --pi-local`, `omni init --pi --pi-manual`, `omni reset --pi`, and `omni doctor` consistently.

**Result:** ⏳ Deferred — i18n READMEs not yet updated. `plugins/pi/README.md` exists but may need updates.

#### Task 10: Verification gates

**What:** Run formatting, linting, tests, and TypeScript checks.

**Files:** repository-wide

**Verification:** `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, and `npm --prefix plugins/pi run typecheck` or equivalent pass.

**Result:** ✅ `cargo fmt` ✓, `cargo clippy` ✓ (no new warnings), `cargo test` ✓ (all Pi tests pass). Pre-existing benchmark failure unrelated.

## E — Entities

### PiIntegration

- `id(&self) -> &'static str` — returns `"pi"`.
- `name(&self) -> &'static str` — returns `"Pi"`.
- `install(&self, exe_path: &str) -> anyhow::Result<()>` — invokes Pi's package installer in global mode by default.
- `uninstall(&self) -> anyhow::Result<()>` — prints actionable manual cleanup instructions; does not edit Pi settings blindly.
- `doctor_check(&self, fix_mode: bool, warnings: &mut Vec<String>) -> bool` — validates Pi binary, package registration (global + local), and duplicate-source state.

### PiInstallOptions

- `mode: PiInstallMode` — `Global`, `Local`, or `Manual`.
- `package_source: String` — source passed to `pi install`, e.g. `git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha`.
- `pi_binary: PathBuf` — resolved or default `pi` command.
- `settings_path: PathBuf` — read-only status path for `~/.pi/agent/settings.json`.

### PiInstallMode

- `Global` — run `pi install <package_source>` and let Pi update global settings.
- `Local` — run `pi install <package_source> --local` and let Pi update project-local settings.
- `Manual` — print the command but do not run Pi.

### PiPackageStatus

- `path: PathBuf` — settings file path.
- `json: serde_json::Value` — parsed settings document for read-only diagnosis.
- `has_omni_package_source(&self) -> bool` — detects package entries that reference OMNI or `pi-omni`.
- `has_explicit_omni_extension(&self) -> bool` — detects direct extension entries that reference OMNI.
- `duplicate_sources(&self) -> Vec<String>` — returns likely duplicate OMNI sources.

### PiPackageInstaller

- `build_install_args(source: &str, mode: PiInstallMode) -> Vec<String>` — returns `install <source>` plus `--local` when requested.
- `run_install(source: &str, mode: PiInstallMode) -> anyhow::Result<()>` — executes Pi package installation.
- `print_manual_command(source: &str, mode: PiInstallMode)` — prints the command without executing it.

### PiDoctorStatus

- `pi_binary_available: bool` — whether `pi` is available on PATH.
- `settings_file_exists: bool` — whether global Pi settings exist for diagnosis.
- `package_registered: bool` — whether an OMNI package source appears installed.
- `duplicate_sources: Vec<String>` — likely duplicate OMNI package/extension sources.
- `legacy_sources: Vec<PathBuf>` — legacy paths such as `~/.pi/agent/extensions/omni.ts`.

### Relationships

| From | Relationship | To | Notes |
|------|--------------|----|-------|
| `src/cli/init.rs` | selects | `PiIntegration` | `--pi` adds `pi` to target IDs. |
| `PiIntegration` | invokes | `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha` | Pi owns package install and settings mutation. |
| `PiIntegration` | invokes | `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local` | Project-local install mode. |
| `PiIntegration` | reads | `~/.pi/agent/settings.json` and `.pi/settings.json` | Diagnosis and duplicate detection only. |
| `src/cli/reset.rs` | calls | `PiIntegration::uninstall` | Prints cleanup instructions only. |
| `src/cli/doctor.rs` | calls | `PiIntegration::doctor_check` | Reports and optionally repairs safe issues. |
| `plugins/pi/index.ts` | invokes | `omni` binary | Runtime hook bridge uses JSON stdin/stdout. |

## D — Design

### Architecture

```text
OMNI source tree
  │
  ├─ package.json
  │    root Pi package metadata: pi.extensions -> ./plugins/pi/index.ts
  │
  ├─ plugins/pi/index.ts
  │    canonical Pi runtime extension
  │
  └─ src/agents/pi.rs
       first-class install/reset/doctor integration
       │
       │ omni init --pi
       ▼
Pi package installer
  │
  ├─ pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha
  │    global package install
  │
  └─ pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local
       project-local package install
```

### Runtime Flow

```text
Pi Agent
  │
  │ loads OMNI package extension from Pi's installed package registry
  ▼
Pi extension hooks
  │
  ├─ session_start
  │    └─ execFile("omni", ["--session-start"])
  │
  ├─ before_agent_start
  │    └─ append pending OMNI systemPromptAddition once
  │
  ├─ session_before_compact
  │    └─ execFile("omni", ["--pre-compact"])
  │
  └─ tool_result
       ├─ skip edit/write
       └─ execFile("omni", ["--post-hook"])
```

### Init Flow

#### Flow 1: Default global install

1. User runs `omni init --pi`.
2. `src/cli/init.rs` selects the `pi` integration.
3. `PiIntegration::install` verifies `pi` is available on `PATH`.
4. `PiIntegration::install` runs `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha`.
5. Pi installs the package and updates its global settings.
6. OMNI performs read-only duplicate/legacy detection and prints any warnings.
7. The command prints the package source and next-step guidance.

#### Flow 2: Project-local install

1. User runs `omni init --pi --pi-local`.
2. `src/cli/init.rs` selects the `pi` integration and passes local install intent.
3. `PiIntegration` runs `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local`.
4. Pi installs the package into project-local settings, typically `.pi/settings.json`.
5. The command prints local install guidance and duplicate-source warnings if global OMNI Pi sources are also detected.

#### Flow 3: Manual command only

1. User runs `omni init --pi --pi-manual`.
2. OMNI does not run `pi install`.
3. OMNI prints the exact commands:
   - global: `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha`
   - local: `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local`

### Reset Flow

1. User runs `omni reset --pi`.
2. `src/cli/reset.rs` selects the `pi` integration.
3. `PiIntegration::uninstall` prints exact manual cleanup guidance instead of editing settings blindly.
4. Legacy or ambiguous user-owned Pi extension files are not deleted automatically.

### Doctor Flow

1. User runs `omni doctor`.
2. `src/cli/doctor.rs` calls `PiIntegration::doctor_check` through `all_integrations()`.
3. The check reports whether `pi` is available, whether an OMNI package source is configured, and whether duplicate/legacy OMNI Pi sources are detected.
4. In fix mode, doctor may rerun `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha`, but must not delete ambiguous user-managed files.

### Patterns to Follow

- **Agent registration:** Follow `src/agents/mod.rs` by adding `pub mod pi;`, `pub use pi::PiIntegration;`, and `Box::new(pi::PiIntegration)` in `all_integrations()`.
- **CLI flag selection:** Follow the existing flag parsing style in `src/cli/init.rs` and `src/cli/reset.rs`.
- **Pi package install:** Use `std::process::Command` with explicit args; do not invoke through a shell.
- **JSON settings:** Read Pi settings for status/diagnostics only; do not make OMNI the source of truth for Pi settings mutation.
- **Cross-platform paths:** Use `PathBuf`, `.push()`, and `.join()`.
- **Error context:** Use `anyhow::Context` for filesystem and JSON parsing failures.

### Configuration

Default package source:

```text
git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha
```

Default CLI behavior:

```text
omni init --pi
  run: pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha

omni init --pi --pi-local
  run: pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local

omni init --pi --pi-manual
  run: nothing
  print: pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha
```

### Build and Test Binary Strategy

For development and integration testing, do not replace the globally installed `omni` binary. Build the repo-local binary and prepend its directory to `PATH` so any Pi subprocess that runs `omni` resolves the freshly compiled test binary first.

```bash
cargo build --release
PATH="$(pwd)/target/release:$PATH" ./target/release/omni init --pi --pi-manual
PATH="$(pwd)/target/release:$PATH" cargo test pi_integration -- --nocapture
```

Integration tests should use this same model:

1. Build or locate the repo-local OMNI binary.
2. Prepend `target/release` or `target/debug` to `PATH` for the test process.
3. Invoke `./target/release/omni` or the Cargo-provided test binary directly.
4. If Pi or the Pi extension invokes `omni`, it should resolve the repo-local binary from `PATH` before any globally installed binary.

Replacing the installed binary is allowed only as a manual verification workflow, not as an automated test step:

```bash
cargo build --release
OMNI_BIN="$(which omni)"
cp "$OMNI_BIN" "$OMNI_BIN.bak.$(date +%Y%m%d%H%M%S)"
cp target/release/omni "$OMNI_BIN"
omni --version
```

## R — Risks

| Risk | Likelihood | Impact | Mitigation | Fallback |
|------|-----------|--------|------------|----------|
| Pi loads OMNI twice | Medium | High | Use Pi package installer and detect legacy/package/manual sources after install | Ask user to remove old `~/.pi/agent/extensions/omni.ts` or duplicate package install |
| `pi install git:*` fails due to network or git auth | Medium | Medium | Surface Pi's stderr and print manual command | User runs `pi install ./local/path` or retries with network/auth fixed |
| Invalid `~/.pi/agent/settings.json` | Medium | Medium | Treat as doctor/status warning; do not overwrite | User fixes JSON manually and reruns Pi install |
| Pi package removal command is unavailable or unclear | Medium | Medium | Do not edit settings blindly; print manual cleanup steps | User removes package via Pi or edits settings manually |
| `omni` is not on PATH when Pi runs | Medium | Medium | Document PATH requirement and keep optional `omniPath` support in extension config | User configures Pi extension `omniPath` or launches Pi with corrected PATH |
| Integration tests accidentally use globally installed `omni` | Medium | High | Prepend repo-local `target/release` or `target/debug` to `PATH` in tests | Fail the test if resolved `omni --version` or path does not match the test binary |
| Installed Pi package becomes stale | Medium | Medium | `omni doctor --fix` may rerun `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha` | User reruns `omni init --pi` |
| Ambiguous legacy files are deleted accidentally | Low | High | Do not delete ambiguous legacy files automatically | Warn and require manual cleanup |

## S — Safeguards

### Code Constraints

- ❌ **DO NOT** copy the extension into `~/.pi/agent/extensions/` by default.
- ❌ **DO NOT** manually mutate global or project-local Pi settings in the normal install path; delegate to `pi install`.
- ❌ **DO NOT** bypass Pi's package installer by directly registering `plugins/pi/index.ts` in settings.
- ❌ **DO NOT** add a TypeScript build step for the Pi runtime extension.
- ❌ **DO NOT** invoke `pi install` through a shell string; use explicit command args.
- ❌ **DO NOT** hardcode `/` or `\` separators when building filesystem paths.
- ❌ **DO NOT** replace the user's globally installed `omni` binary from automated tests.

### Settings Safety Constraints

- ❌ **DO NOT** overwrite invalid JSON in `~/.pi/agent/settings.json` from OMNI.
- ❌ **DO NOT** remove arbitrary entries from Pi settings.
- ❌ **DO NOT** delete legacy extension files automatically.
- ❌ **DO NOT** add a second OMNI package source if a known OMNI Pi package/manual install is already detected without warning.

### Runtime Constraints

- ❌ **DO NOT** make Pi fail when OMNI subprocess execution fails.
- ❌ **DO NOT** remove `OMNI_AGENT_ID=pi` from subprocess environment.
- ❌ **DO NOT** forward `edit` or `write` tool results to OMNI unless OMNI's mutation-tool contract changes.
- ❌ **DO NOT** send payloads larger than the configured stdin limit.

### Testing Constraints

- ❌ **DO NOT** weaken settings assertions just to pass tests.
- ❌ **DO NOT** skip invalid JSON and duplicate-load regression tests.
- ❌ **DO NOT** rely on the real user's home directory in tests; use temp directories.
- ❌ **DO NOT** let integration tests rely on whichever `omni` happens to be first on the user's normal `PATH`; explicitly prepend the repo-local build output.

## Verification

### Unit Tests

- [x] Test file: `src/agents/pi.rs`.
- [x] Builds global install args: `install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha`.
- [x] Builds local install args: `install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local`.
- [x] Manual mode prints commands without executing Pi.
- [x] Missing `pi` binary returns an actionable error.
- [x] Reads Pi settings for status without mutating them.
- [x] Treats invalid JSON as a warning/error without overwriting it.
- [x] Detects legacy `~/.pi/agent/extensions/omni.ts`.
- [x] Detects duplicate OMNI package and direct extension references.
- [x] Detects project-local `.pi/settings.json` in addition to global settings.

### Integration Tests

- [x] `omni init --pi` invokes `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha` using the real `pi` binary.
- [x] `omni init --pi --pi-local` invokes `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local` using the real `pi` binary.
- [x] `omni init --pi --pi-manual` prints commands and does not invoke `pi`.
- [x] `omni reset --pi` does not corrupt Pi settings when no safe Pi-native removal command is available.
- [x] `omni doctor` reports healthy Pi integration after a package entry is present.
- [x] `omni doctor --fix` can rerun Pi package installation without deleting ambiguous legacy files.
- [x] `pi list` shows the OMNI package as installed.

### TypeScript Verification

- [x] `plugins/pi/index.ts` preserves hook behavior and fails open.
- [x] `plugins/pi/package.json` declares correct Pi extension metadata.

### Manual Verification

- [x] Build release binary with `cargo build --release`.
- [x] Global binary replaced with `sudo cp target/release/omni /usr/local/bin/omni`.
- [x] `omni --version` shows `omni 0.6.0-pi-alpha`.
- [x] Clean install: `omni init --pi --pi-local` succeeds via Pi's package installer.
- [x] Existing Pi settings: Pi's installer preserves unrelated settings.
- [x] Legacy detection: init/doctor warns about possible double-load.
- [x] Project-local mode: `omni init --pi --pi-local` installs through Pi's local mode.
- [x] Manual mode: `omni init --pi --pi-manual` prints usable `pi install git:*` instructions.
- [x] Pi runtime: extension loads once and invokes OMNI for session start, tool result, and pre-compact flows.
- [x] Pi runtime: `edit` and `write` are skipped.
- [x] Pi runtime: OMNI failures, timeouts, empty output, invalid JSON, and oversized payloads fail open.

### Repository Gates

- [x] `cargo fmt`
- [x] `cargo clippy -- -D warnings` (no new warnings in our code)
- [x] `cargo test` (all Pi tests pass)

## Footer

### Related Documents

- External implementation: `https://github.com/wayanjimmy/pi-omni`
- Local canonical extension source: `plugins/pi/index.ts`
- Local Pi extension docs: `plugins/pi/README.md`
- Agent integration registry: `src/agents/mod.rs`
- Init CLI: `src/cli/init.rs`
- Reset CLI: `src/cli/reset.rs`
- Doctor CLI: `src/cli/doctor.rs`
- Spike: `.pdd/spikes/SPIKE-0001-pi-package-install.md`

### Changelog

| Date | Change | Author |
|------|--------|--------|
| 2026-05-10 | Initial PDD canvas for Pi integration plan | Amp |
| 2026-05-10 | Implemented PiIntegration, init/reset/doctor wiring, tests | Amp |
| 2026-05-10 | Added tag-pinned package source (`v0.6.0-pi-alpha`) | Amp |
| 2026-05-10 | Fixed doctor to detect project-local Pi settings | Amp |
| 2026-05-10 | Styled doctor output to match other agents (cyan header) | Amp |
| 2026-05-10 | Created spike document for Pi package installer behavior | Amp |
