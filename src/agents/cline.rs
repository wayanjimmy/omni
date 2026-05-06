use crate::agents::AgentIntegration;
use colored::*;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

pub struct ClineIntegration;

impl AgentIntegration for ClineIntegration {
    fn id(&self) -> &'static str {
        "cline"
    }

    fn name(&self) -> &'static str {
        "Cline"
    }

    fn install(&self, exe_path: &str) -> anyhow::Result<()> {
        let settings_path = get_cline_path();
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut val = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        if let Some(obj) = val.as_object_mut() {
            let mcp_servers = obj.entry("mcpServers").or_insert_with(|| json!({}));
            if let Some(servers) = mcp_servers.as_object_mut() {
                servers.insert(
                    "omni".to_string(),
                    json!({
                        "type": "stdio",
                        "command": exe_path,
                        "args": ["--mcp"],
                        "disabled": false,
                        "env": { "OMNI_AGENT_ID": "cline" }
                    }),
                );
            }
        }

        install_omni_hooks(&mut val, exe_path);
        fs::write(&settings_path, serde_json::to_string_pretty(&val)?)?;
        println!(
            "  {} Configured {} + {} in Cline settings",
            "✓".green(),
            "MCP Server".bold(),
            "Hooks".bold()
        );
        Ok(())
    }

    fn uninstall(&self) -> anyhow::Result<()> {
        let settings_path = get_cline_path();
        if !settings_path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&settings_path)?;
        let Ok(mut val) = serde_json::from_str::<Value>(&content) else {
            return Ok(());
        };

        if let Some(obj) = val.as_object_mut()
            && let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut())
        {
            servers.remove("omni");
        }
        remove_omni_hooks(&mut val);

        fs::write(&settings_path, serde_json::to_string_pretty(&val)?)?;
        println!(
            "  {} Removed MCP Server + Hooks from Cline settings",
            "✓".yellow()
        );
        Ok(())
    }

    fn doctor_check(&self, fix_mode: bool, warnings: &mut Vec<String>) -> bool {
        let settings_path = get_cline_path();
        let mut all_ok = true;

        println!("\n  {}", "Cline AI:".cyan());

        if !settings_path.exists() {
            if fix_mode {
                if let Ok(exe_path) = std::env::current_exe() {
                    let _ = self.install(&exe_path.to_string_lossy());
                }
                println!(
                    "   {:<15} {}",
                    "Config:".bright_black(),
                    "[FIXED] installed".green().bold()
                );
                return true;
            }
            println!(
                "   {:<15} {}",
                "Config:".bright_black(),
                "not configured".bright_black()
            );
            return false;
        }

        let content = fs::read_to_string(&settings_path).unwrap_or_default();

        // Check MCP
        if content.contains("\"omni\"") {
            println!(
                "   {:<15} {} {}",
                "MCP Server:".bright_black(),
                settings_path.display().to_string().bright_black(),
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
                    "MCP Server:".bright_black(),
                    "[FIXED] registered".green().bold()
                );
            } else {
                println!(
                    "   {:<15} {}",
                    "MCP Server:".bright_black(),
                    "[WARNING] not configured".yellow().bold()
                );
                warnings
                    .push("Cline MCP server not configured. Run `omni init --cline`.".to_string());
            }
        }

        // Check hooks
        let has_hooks = content.contains("PreToolUse") && content.contains("PostToolUse");
        if has_hooks {
            println!(
                "   {:<15} {}",
                "PreToolUse".bright_black(),
                "[OK] installed".green()
            );
            println!(
                "   {:<15} {}",
                "PostToolUse".bright_black(),
                "[OK] installed".green()
            );
        } else {
            all_ok = false;
            if fix_mode {
                if let Ok(exe_path) = std::env::current_exe() {
                    let _ = self.install(&exe_path.to_string_lossy());
                }
                println!(
                    "   {:<15} {}",
                    "Hooks:".bright_black(),
                    "[FIXED] missing hooks installed".green().bold()
                );
            } else {
                println!(
                    "   {:<15} {}",
                    "Hooks:".bright_black(),
                    "[WARNING] hooks not configured".yellow().bold()
                );
                warnings.push("Cline hooks not configured. Run `omni init --cline`.".to_string());
            }
        }

        all_ok
    }
}

fn get_cline_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
    }
    #[cfg(target_os = "linux")]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        PathBuf::from("cline_mcp_settings.json")
    }
}

pub fn install_omni_hooks(val: &mut Value, exe_path: &str) {
    let obj = match val.as_object_mut() {
        Some(o) => o,
        None => {
            *val = json!({});
            val.as_object_mut().unwrap()
        }
    };
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .unwrap();
    if exe_path.is_empty() {
        return;
    }

    let ensure_hook = |arr_val: &mut Value, matcher: &str, hook_cmd: &str| {
        let arr = arr_val.as_array_mut().unwrap();
        for v in arr.iter() {
            if let Some(inner) = v.get("hooks").and_then(|h| h.as_array()) {
                for h in inner {
                    if h.get("command").and_then(|c| c.as_str()) == Some(hook_cmd) {
                        return;
                    }
                }
            }
        }
        arr.push(
            json!({ "matcher": matcher, "hooks": [{ "type": "command", "command": hook_cmd }] }),
        );
    };

    let pre_cmd = format!("{} --pre-hook", exe_path);
    let post_cmd = format!("{} --post-hook", exe_path);
    let compact_cmd = format!("{} --pre-compact", exe_path);

    ensure_hook(
        hooks.entry("PreToolUse").or_insert_with(|| json!([])),
        "Bash",
        &pre_cmd,
    );
    ensure_hook(
        hooks.entry("PostToolUse").or_insert_with(|| json!([])),
        "Bash",
        &post_cmd,
    );
    ensure_hook(
        hooks.entry("PreCompact").or_insert_with(|| json!([])),
        "",
        &compact_cmd,
    );
}

pub fn remove_omni_hooks(val: &mut Value) {
    if let Some(obj) = val.as_object_mut()
        && let Some(hooks) = obj.get_mut("hooks").and_then(|h| h.as_object_mut())
    {
        for (_key, arr_val) in hooks.iter_mut() {
            if let Some(arr) = arr_val.as_array_mut() {
                arr.retain(|v| {
                    if let Some(inner) = v.get("hooks").and_then(|h| h.as_array()) {
                        !inner.iter().any(|h| {
                            h.get("command").and_then(|c| c.as_str()).is_some_and(|c| {
                                c.contains("omni")
                                    && (c.contains("--hook")
                                        || c.contains("--post-hook")
                                        || c.contains("--pre-hook")
                                        || c.contains("--pre-compact"))
                            })
                        })
                    } else {
                        true
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_hooks_creates_valid_structure() {
        let mut val = json!({});
        install_omni_hooks(&mut val, "/usr/bin/omni");
        let hooks = val.get("hooks").unwrap().as_object().unwrap();
        assert!(hooks.contains_key("PreToolUse"));
        assert!(hooks.contains_key("PostToolUse"));
        assert!(hooks.contains_key("PreCompact"));
    }

    #[test]
    fn test_install_hooks_idempotent() {
        let mut val = json!({});
        install_omni_hooks(&mut val, "/usr/bin/omni");
        let count = |v: &Value, key: &str| -> usize {
            v.get("hooks")
                .unwrap()
                .get(key)
                .unwrap()
                .as_array()
                .unwrap()
                .len()
        };
        assert_eq!(count(&val, "PreToolUse"), 1);
        install_omni_hooks(&mut val, "/usr/bin/omni");
        assert_eq!(count(&val, "PreToolUse"), 1, "Should be idempotent");
    }

    #[test]
    fn test_remove_hooks_cleans_entries() {
        let mut val = json!({});
        install_omni_hooks(&mut val, "/usr/bin/omni");
        assert!(
            !val.get("hooks")
                .unwrap()
                .get("PreToolUse")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        remove_omni_hooks(&mut val);
        assert_eq!(
            val.get("hooks")
                .unwrap()
                .get("PreToolUse")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
}
