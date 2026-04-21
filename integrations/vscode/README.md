# OMNI for VS Code

Three integration methods for using OMNI's intelligent output distillation in VS Code.

## Method 1: Shell Hook (Recommended — Zero Config)

OMNI works automatically in VS Code terminal via magic pipe detection (PGID-based).

Install OMNI, then in VS Code terminal:

```bash
export OMNI_AGENT=vscode
npm test  # automatically distilled by OMNI if piped
```

Or pipe explicitly:

```bash
cargo build 2>&1 | omni
omni exec npm test
```

## Method 2: Task Runner Integration

Copy `omni_tasks.json` to your project's `.vscode/tasks.json`:

```bash
mkdir -p .vscode
cp integrations/vscode/omni_tasks.json .vscode/tasks.json
```

Then use `Cmd+Shift+B` (Build) or `Cmd+Shift+P → "Run Test Task"` to run through OMNI.

### Available Tasks:

| Task | Shortcut | Description |
|---|---|---|
| OMNI: Build | `Cmd+Shift+B` | `cargo build` through OMNI |
| OMNI: Test | Test Task | `cargo test` through OMNI |
| OMNI: Stats | Manual | View token savings |

## Method 3: GitHub Copilot Chat Integration

Copy `copilot-instructions.md` to your project's `.vscode/` and add to VS Code settings:

```json
{
  "github.copilot.chat.codeGeneration.instructions": [
    {
      "file": ".vscode/copilot-instructions.md"
    }
  ]
}
```

This instructs Copilot to pipe terminal commands through OMNI automatically.

## Token Savings

View savings with: `omni stats`
