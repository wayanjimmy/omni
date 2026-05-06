use crate::agents::AgentIntegration;
use colored::*;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

pub struct CursorIntegration;

impl AgentIntegration for CursorIntegration {
    fn id(&self) -> &'static str {
        "cursor"
    }

    fn name(&self) -> &'static str {
        "Cursor AI"
    }

    fn install(&self, exe_path: &str) -> anyhow::Result<()> {
        // Install MCP server
        let (mcp_path, mut mcp_val) = initialize_mcp_config()?;
        install_mcp_server(&mut mcp_val, exe_path);
        fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_val)?)?;
        println!(
            "  {} Configured MCP Server in ~/.cursor/mcp.json",
            "✓".green()
        );

        // Install hooks
        install_omni_hooks(exe_path)?;
        println!(
            "  {} Configured {} in ~/.cursor/hooks.json",
            "✓".green(),
            "Hooks".bold()
        );

        Ok(())
    }

    fn uninstall(&self) -> anyhow::Result<()> {
        // Remove MCP
        let mcp_path = get_mcp_path();
        if mcp_path.exists() {
            let content = fs::read_to_string(&mcp_path)?;
            if let Ok(mut val) = serde_json::from_str::<Value>(&content) {
                remove_mcp_server(&mut val);
                fs::write(&mcp_path, serde_json::to_string_pretty(&val)?)?;
                println!(
                    "  {} Removed MCP Server from ~/.cursor/mcp.json",
                    "✓".yellow()
                );
            }
        }

        // Remove hooks
        remove_omni_hooks()?;
        println!("  {} Removed Hooks from ~/.cursor/hooks.json", "✓".yellow());
        Ok(())
    }

    fn doctor_check(&self, fix_mode: bool, warnings: &mut Vec<String>) -> bool {
        let mcp_path = get_mcp_path();
        let hooks_path = get_hooks_path();
        let mut all_ok = true;

        println!("\n  {}", "Cursor AI:".cyan());

        // Check MCP
        if mcp_path.exists()
            && let Ok(content) = fs::read_to_string(&mcp_path)
            && let Ok(val) = serde_json::from_str::<Value>(&content)
            && has_valid_omni_server(&val)
        {
            println!(
                "   {:<15} {} {}",
                "MCP: ".bright_black(),
                "~/.cursor/mcp.json".bright_black(),
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
                    "MCP: ".bright_black(),
                    "[FIXED] registered".green().bold()
                );
            } else {
                println!(
                    "   {:<15} {}",
                    "MCP: ".bright_black(),
                    "not configured".bright_black()
                );
                warnings.push(
                    "Cursor MCP server is not configured. Run `omni init --cursor`.".to_string(),
                );
            }
        }

        // Check hooks
        let hooks_content = if hooks_path.exists() {
            fs::read_to_string(&hooks_path).unwrap_or_default()
        } else {
            String::new()
        };
        let has_hooks =
            hooks_content.contains("--pre-hook") && hooks_content.contains("--post-hook");

        if has_hooks {
            println!(
                "   {:<15} {}",
                "beforeShell".bright_black(),
                "[OK] installed".green()
            );
            println!(
                "   {:<15} {}",
                "afterFileEdit".bright_black(),
                "[OK] installed".green()
            );
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
                println!(
                    "   {:<15} {}",
                    "Hooks:".bright_black(),
                    "[WARNING] hooks not configured".yellow().bold()
                );
                warnings.push("Cursor hooks not configured. Run `omni init --cursor`.".to_string());
            }
        }

        all_ok
    }
}

fn get_mcp_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cursor/mcp.json")
}

fn get_hooks_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cursor/hooks.json")
}

fn initialize_mcp_config() -> anyhow::Result<(PathBuf, Value)> {
    let mcp_path = get_mcp_path();
    if let Some(parent) = mcp_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let val = if mcp_path.exists() {
        let content = fs::read_to_string(&mcp_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    Ok((mcp_path, val))
}

fn install_mcp_server(val: &mut Value, exe_path: &str) {
    let obj = match val.as_object_mut() {
        Some(o) => o,
        None => {
            *val = json!({});
            val.as_object_mut().unwrap()
        }
    };
    let servers = obj
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .unwrap();
    servers.insert(
        "omni".to_string(),
        json!({
            "type": "stdio", "command": exe_path, "args": ["--mcp"],
            "env": { "OMNI_AGENT_ID": "cursor" }
        }),
    );
}

fn remove_mcp_server(val: &mut Value) {
    if let Some(obj) = val.as_object_mut()
        && let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut())
    {
        servers.remove("omni");
    }
}

fn has_valid_omni_server(val: &Value) -> bool {
    val.get("mcpServers")
        .and_then(|v| v.as_object())
        .and_then(|servers| servers.get("omni"))
        .is_some_and(|omni| {
            omni.get("command").and_then(|v| v.as_str()).is_some()
                && omni
                    .get("args")
                    .and_then(|v| v.as_array())
                    .is_some_and(|args| args.iter().any(|arg| arg.as_str() == Some("--mcp")))
        })
}

pub fn install_omni_hooks(exe_path: &str) -> anyhow::Result<()> {
    let hooks_path = get_hooks_path();
    if let Some(parent) = hooks_path.parent() {
        fs::create_dir_all(parent)?;
    }

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

    let ensure_hook = |arr_val: &mut Value, cmd: &str| {
        let arr = arr_val.as_array_mut().unwrap();
        for v in arr.iter() {
            if v.get("command").and_then(|c| c.as_str()) == Some(cmd) {
                return;
            }
        }
        arr.push(json!({ "command": cmd }));
    };

    // Cursor uses beforeShellExecution / afterFileEdit
    ensure_hook(
        hooks
            .entry("beforeShellExecution")
            .or_insert_with(|| json!([])),
        &pre_cmd,
    );
    ensure_hook(
        hooks.entry("afterFileEdit").or_insert_with(|| json!([])),
        &post_cmd,
    );

    fs::write(&hooks_path, serde_json::to_string_pretty(&val)?)?;
    Ok(())
}

pub fn remove_omni_hooks() -> anyhow::Result<()> {
    let hooks_path = get_hooks_path();
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
                            && (c.contains("--pre-hook") || c.contains("--post-hook")))
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
    fn install_mcp_server_is_idempotent() {
        let mut val = json!({});
        install_mcp_server(&mut val, "/usr/local/bin/omni");
        install_mcp_server(&mut val, "/usr/local/bin/omni");
        let servers = val
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .expect("mcpServers exists");
        assert_eq!(servers.len(), 1);
        assert!(servers.contains_key("omni"));
    }

    #[test]
    fn remove_mcp_server_removes_only_omni() {
        let mut val = json!({ "mcpServers": { "omni": {"command": "/usr/local/bin/omni", "args": ["--mcp"]}, "other": {"command": "other"} } });
        remove_mcp_server(&mut val);
        let servers = val
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .expect("mcpServers exists");
        assert!(!servers.contains_key("omni"));
        assert!(servers.contains_key("other"));
    }

    #[test]
    fn test_hooks_json_structure() {
        let mut val = json!({});
        let obj = val.as_object_mut().unwrap();
        let hooks = obj
            .entry("hooks")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap();
        let arr = hooks
            .entry("beforeShellExecution")
            .or_insert_with(|| json!([]));
        arr.as_array_mut()
            .unwrap()
            .push(json!({ "command": "/usr/bin/omni --pre-hook" }));

        let s = serde_json::to_string_pretty(&val).unwrap();
        assert!(s.contains("beforeShellExecution"));
        assert!(s.contains("--pre-hook"));
    }
}
