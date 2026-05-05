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
        let (mcp_path, mut val) = initialize_mcp_config()?;

        install_mcp_server(&mut val, exe_path);
        fs::write(&mcp_path, serde_json::to_string_pretty(&val)?)?;

        println!(
            "  {} Configured MCP Server in ~/.cursor/mcp.json",
            "✓".green()
        );
        Ok(())
    }

    fn uninstall(&self) -> anyhow::Result<()> {
        let mcp_path = get_mcp_path();
        if !mcp_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&mcp_path)?;
        let Ok(mut val) = serde_json::from_str::<Value>(&content) else {
            return Ok(());
        };

        remove_mcp_server(&mut val);
        fs::write(&mcp_path, serde_json::to_string_pretty(&val)?)?;

        println!(
            "  {} Removed MCP Server from ~/.cursor/mcp.json",
            "✓".yellow()
        );
        Ok(())
    }

    fn doctor_check(&self, fix_mode: bool, warnings: &mut Vec<String>) -> bool {
        let mcp_path = get_mcp_path();

        println!("\n  {}", "Cursor AI:".cyan());
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
            return true;
        }

        if fix_mode {
            if let Ok(exe_path) = std::env::current_exe() {
                let _ = self.install(&exe_path.to_string_lossy());
            }
            println!(
                "   {:<15} {}",
                "MCP: ".bright_black(),
                "[FIXED] registered".green().bold()
            );
            true
        } else {
            println!(
                "   {:<15} {}",
                "MCP: ".bright_black(),
                "not configured".bright_black()
            );
            warnings
                .push("Cursor MCP server is not configured. Run `omni init --cursor`.".to_string());
            false
        }
    }
}

fn get_mcp_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cursor/mcp.json")
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
            "type": "stdio",
            "command": exe_path,
            "args": ["--mcp"],
            "env": {
                "OMNI_AGENT_ID": "cursor"
            }
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
        let mut val = json!({
            "mcpServers": {
                "omni": {"command": "/usr/local/bin/omni", "args": ["--mcp"]},
                "other": {"command": "other"}
            }
        });

        remove_mcp_server(&mut val);

        let servers = val
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .expect("mcpServers exists");

        assert!(!servers.contains_key("omni"));
        assert!(servers.contains_key("other"));
    }
}
