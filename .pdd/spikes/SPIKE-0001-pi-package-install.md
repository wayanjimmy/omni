---
status: completed
date: 2026-05-10
timebox: 1 day
authors: [Amp, user]
canvas: .pdd/canvas/0001-pi-extension-integration.md
---

# Spike: Pi Package Installer Behavior

## Timebox

**Start:** 2026-05-10
**End:** 2026-05-10
**Total days:** 1

## Objective

Determine how Pi's `pi install` command works with git sources, whether it supports tags/branches, and how it resolves extension entry points from a cloned repository.

### Success Criteria

- [x] Confirm `pi install git:*` clones and installs from remote
- [x] Determine if `@tag` syntax is supported
- [x] Determine if `#branch` syntax is supported
- [x] Understand how Pi resolves `pi.extensions` from cloned repo
- [x] Understand local path install behavior

## Context

The PDD-0001 canvas assumes `pi install git:github.com/<owner>/omni` is the install mechanism. We need to validate this assumption and understand edge cases (tag pinning, branch selection, local dev workflow).

## Investigation Plan

### Step 1: Basic git source install

- [x] Run `pi install git:github.com/wayanjimmy/omni --local`
- [x] Verify `.pi/settings.json` is updated
- [x] Verify `.pi/git/` contains cloned repo

### Step 2: Tag syntax

- [x] Run `pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local`
- [x] Verify tag is checked out after clone

### Step 3: Branch syntax

- [x] Run `pi install git:github.com/wayanjimmy/omni#feat/pi-extension-integration --local`
- [x] Confirm `#branch` does NOT work (fragment passed to git URL)

### Step 4: Local path install

- [x] Run `pi install ./ --local`
- [x] Verify relative path stored in settings
- [x] Verify extension loads from local filesystem

## Findings

### What Worked

1. **`pi install git:*` works as expected** — clones repo, checks out default branch, reads root `package.json` for `pi.extensions`.

2. **`@tag` syntax IS supported** — Pi does `git checkout <tag>` after clone. Verified with `v0.6.0-pi-alpha`.

3. **Local path install works** — `pi install ./ --local` stores `".."` (relative path) in `.pi/settings.json` and resolves extension at runtime.

### What Didn't Work

1. **`#branch` syntax fails** — The fragment is appended to the git URL, causing a clone error:
   ```
   fatal: remote error: ... is not a valid repository name
   ```
   Pi does not parse `#branch` as a branch selector.

2. **Local path stores relative path** — Installing from `./` stores `".."` in settings, which is fragile if the working directory changes.

### Dead Ends

- **Using `#branch` for feature branch installs:** Rejected. Must use tags or install from local path during development.

## Architecture

### Install Flow (Validated)

```
User runs: pi install git:github.com/<owner>/omni@v0.6.0-pi-alpha --local
  │
  ▼
Pi clones: https://github.com/<owner>/omni
  │
  ▼
Pi checks out: v0.6.0-pi-alpha (git checkout <tag>)
  │
  ▼
Pi reads: <clone>/package.json → "pi.extensions": ["./plugins/pi/index.ts"]
  │
  ▼
Pi stores: "git:github.com/<owner>/omni@v0.6.0-pi-alpha" in .pi/settings.json
```

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Tag becomes stale | Medium | Low | `omni doctor --fix` reruns install; user can also re-tag |
| Relative path in local install | Medium | Low | Document that local path installs are dev-only |
| No branch support | Low | Low | Use tags for releases, local path for dev |

## Recommendation

### Chosen Approach

Pin the default package source to a tag: `git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha`

This gives deterministic installs. For development, use `OMNI_PI_PACKAGE_SOURCE=./ omni init --pi` or `pi install ./ --local`.

### Next Steps

1. Update `DEFAULT_PACKAGE_SOURCE` in `src/agents/pi.rs` to use tag-pinned source
2. Document tag-based install in README
3. Re-tag on each release

## Appendix

### Version Numbers

| Technology | Version | Notes |
|------------|---------|-------|
| pi CLI | latest | `pi install` supports `@tag`, not `#branch` |
| omni | 0.6.0-pi-alpha | Fork version with Pi integration |

### Commands Verified

```bash
# Remote tag install
pi install git:github.com/wayanjimmy/omni@v0.6.0-pi-alpha --local

# Local path install (dev)
pi install ./ --local

# List installed packages
pi list
```
