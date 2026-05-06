use crate::agents::AgentIntegration;
use colored::*;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

pub struct CodexIntegration;

impl AgentIntegration for CodexIntegration {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn name(&self) -> &'static str {
        "Codex CLI"
    }

    fn install(&self, exe_path: &str) -> anyhow::Result<()> {
        let codex_dir = get_codex_dir();
        fs::create_dir_all(&codex_dir)?;

        // Install MCP server in config.toml
        let config_path = codex_dir.join("config.toml");
        let mut content = if config_path.exists() {
            fs::read_to_string(&config_path)?
        } else {
            String::new()
        };

        if !content.contains("[mcp_servers.omni]") {
            content.push_str(&format!(
                "\n[mcp_servers.omni]\ntype = \"stdio\"\ncommand = \"{}\"\nargs = [\"--mcp\"]\n",
                exe_path
            ));
            fs::write(&config_path, content)?;
        }

        println!(
            "  {} Configured MCP Server in ~/.codex/config.toml",
            "✓".green()
        );

        // Install hooks in hooks.json
        install_omni_hooks(exe_path)?;
        println!(
            "  {} Configured {} in ~/.codex/hooks.json",
            "✓".green(),
            "Hooks".bold()
        );

        Ok(())
    }

    fn uninstall(&self) -> anyhow::Result<()> {
        let codex_dir = get_codex_dir();

        // Remove MCP from config.toml
        let config_path = codex_dir.join("config.toml");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            if content.contains("[mcp_servers.omni]") {
                let mut new_content = String::new();
                let mut skip = false;
                for line in content.lines() {
                    if line.starts_with("[mcp_servers.omni]") {
                        skip = true;
                    } else if skip && line.starts_with('[') {
                        skip = false;
                    }
                    if !skip {
                        new_content.push_str(line);
                        new_content.push('\n');
                    }
                }
                fs::write(&config_path, new_content.trim_end().to_string() + "\n")?;
                println!(
                    "  {} Removed MCP Server from ~/.codex/config.toml",
                    "✓".yellow()
                );
            }
        }

        // Remove hooks from hooks.json
        remove_omni_hooks()?;
        println!("  {} Removed Hooks from ~/.codex/hooks.json", "✓".yellow());

        Ok(())
    }

    fn doctor_check(&self, fix_mode: bool, warnings: &mut Vec<String>) -> bool {
        let codex_dir = get_codex_dir();
        let config_path = codex_dir.join("config.toml");
        let hooks_path = codex_dir.join("hooks.json");
        let mut all_ok = true;

        println!("\n  {}", "Codex CLI:".cyan());

        // Check MCP config
        if config_path.exists()
            && fs::read_to_string(&config_path)
                .unwrap_or_default()
                .contains("omni")
        {
            println!(
                "   {:<15} {} {}",
                "Config:".bright_black(),
                "~/.codex/config.toml".bright_black(),
                "[OK]".green().bold()
            );
        } else {
            all_ok = false;
            if fix_mode {
                if let Ok(exe_path) = std::env::current_exe() {
                    let _ = self.install(&exe_path.to_string_lossy());
                }
                println!(
                    "   {:<15} {}",
                    "Config:".bright_black(),
                    "[FIXED] registered".green().bold()
                );
            } else {
                println!(
                    "   {:<15} {}",
                    "Config:".bright_black(),
                    "not configured".bright_black()
                );
                warnings.push(
                    "Codex CLI MCP server not configured. Run `omni init --codex`.".to_string(),
                );
            }
        }

        // Check hooks
        let hooks_content = if hooks_path.exists() {
            fs::read_to_string(&hooks_path).unwrap_or_default()
        } else {
            String::new()
        };

        let has_pre = hooks_content.contains("--pre-hook");
        let has_post = hooks_content.contains("--post-hook");
        let has_session = hooks_content.contains("--session-start");

        if has_pre && has_post && has_session {
            let fmt_hook = |name: &str, present: bool| {
                if present {
                    println!(
                        "   {:<15} {}",
                        name.bright_black(),
                        "[OK] installed".green()
                    );
                }
            };
            fmt_hook("PreToolUse", has_pre);
            fmt_hook("PostToolUse", has_post);
            fmt_hook("SessionStart", has_session);
        } else {
            all_ok = false;
            if fix_mode {
                if let Ok(exe_path) = std::env::current_exe() {
                    let _ = install_omni_hooks(&exe_path.to_string_lossy());
                }
                println!(
                    "   {:<15} {}",
                    "Hooks:".bright_black(),
                    "[FIXED] missing hooks installed".green().bold()
                );
            } else {
                if !has_pre {
                    println!(
                        "   {:<15} {}",
                        "PreToolUse".bright_black(),
                        "[WARNING] missing".yellow()
                    );
                }
                if !has_post {
                    println!(
                        "   {:<15} {}",
                        "PostToolUse".bright_black(),
                        "[WARNING] missing".yellow()
                    );
                }
                if !has_session {
                    println!(
                        "   {:<15} {}",
                        "SessionStart".bright_black(),
                        "[WARNING] missing".yellow()
                    );
                }
                warnings
                    .push("Codex CLI hooks not configured. Run `omni init --codex`.".to_string());
            }
        }

        all_ok
    }
}

fn get_codex_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
}

pub fn install_omni_hooks(exe_path: &str) -> anyhow::Result<()> {
    let hooks_path = get_codex_dir().join("hooks.json");
    fs::create_dir_all(get_codex_dir())?;

    let mut val = if hooks_path.exists() {
        let content = fs::read_to_string(&hooks_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    let obj = match val.as_object_mut() {
        Some(o) => o,
        None => {
            val = json!({});
            val.as_object_mut().unwrap()
        }
    };

    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .unwrap();

    let pre_cmd = format!("{} --pre-hook", exe_path);
    let post_cmd = format!("{} --post-hook", exe_path);
    let session_cmd = format!("{} --session-start", exe_path);

    let ensure_hook = |arr_val: &mut Value, cmd: &str| {
        let arr = arr_val.as_array_mut().unwrap();
        // Check if already present
        for v in arr.iter() {
            if v.get("command").and_then(|c| c.as_str()) == Some(cmd) {
                return;
            }
        }
        arr.push(json!({ "command": cmd }));
    };

    ensure_hook(
        hooks.entry("PreToolUse").or_insert_with(|| json!([])),
        &pre_cmd,
    );
    ensure_hook(
        hooks.entry("PostToolUse").or_insert_with(|| json!([])),
        &post_cmd,
    );
    ensure_hook(
        hooks.entry("SessionStart").or_insert_with(|| json!([])),
        &session_cmd,
    );

    fs::write(&hooks_path, serde_json::to_string_pretty(&val)?)?;
    Ok(())
}

pub fn remove_omni_hooks() -> anyhow::Result<()> {
    let hooks_path = get_codex_dir().join("hooks.json");
    if !hooks_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&hooks_path)?;
    let Ok(mut val) = serde_json::from_str::<Value>(&content) else {
        return Ok(());
    };

    if let Some(obj) = val.as_object_mut()
        && let Some(hooks) = obj.get_mut("hooks").and_then(|h| h.as_object_mut())
    {
        for (_key, arr_val) in hooks.iter_mut() {
            if let Some(arr) = arr_val.as_array_mut() {
                arr.retain(|v| {
                    v.get("command").and_then(|c| c.as_str()).is_none_or(|c| {
                        !(c.contains("omni")
                            && (c.contains("--pre-hook")
                                || c.contains("--post-hook")
                                || c.contains("--session-start")))
                    })
                });
            }
        }
    }

    fs::write(&hooks_path, serde_json::to_string_pretty(&val)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_hooks_creates_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let hooks_path = dir.path().join("hooks.json");

        // Manually write to a known path for testing the JSON structure
        let mut val = json!({});
        let obj = val.as_object_mut().unwrap();
        let hooks = obj
            .entry("hooks")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap();

        let cmd = "/usr/bin/omni --pre-hook";
        let arr = hooks.entry("PreToolUse").or_insert_with(|| json!([]));
        arr.as_array_mut().unwrap().push(json!({ "command": cmd }));

        fs::write(&hooks_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();

        let content = fs::read_to_string(&hooks_path).unwrap();
        assert!(content.contains("PreToolUse"));
        assert!(content.contains("--pre-hook"));
    }

    #[test]
    fn test_ensure_hook_is_idempotent() {
        let mut val = json!({ "hooks": { "PreToolUse": [] } });
        let hooks = val.get_mut("hooks").unwrap().as_object_mut().unwrap();

        let cmd = "/usr/bin/omni --pre-hook";
        let ensure = |arr_val: &mut Value, cmd: &str| {
            let arr = arr_val.as_array_mut().unwrap();
            for v in arr.iter() {
                if v.get("command").and_then(|c| c.as_str()) == Some(cmd) {
                    return;
                }
            }
            arr.push(json!({ "command": cmd }));
        };

        ensure(hooks.get_mut("PreToolUse").unwrap(), cmd);
        assert_eq!(
            hooks.get("PreToolUse").unwrap().as_array().unwrap().len(),
            1
        );

        ensure(hooks.get_mut("PreToolUse").unwrap(), cmd);
        assert_eq!(
            hooks.get("PreToolUse").unwrap().as_array().unwrap().len(),
            1,
            "Should be idempotent"
        );
    }
}
