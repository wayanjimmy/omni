pub fn command_family(cmd: &str) -> String {
    let c = cmd.trim();
    if c.is_empty() {
        return "unknown".to_string();
    }

    let mut parts = c.split_whitespace();
    let first = parts.next().unwrap_or("");
    let second = parts.next().unwrap_or("");

    match first {
        "git" => match second {
            "status" | "diff" | "log" | "show" | "grep" | "blame" => format!("git {}", second),
            _ => "git".to_string(),
        },
        "cargo" => match second {
            "build" | "test" | "check" | "run" | "clippy" => format!("cargo {}", second),
            _ => "cargo".to_string(),
        },
        "npm" | "yarn" | "pnpm" | "bun" => match second {
            "install" | "test" | "build" | "run" | "lint" => format!("{} {}", first, second),
            _ => first.to_string(),
        },
        "kubectl" => match second {
            "get" | "describe" | "logs" | "apply" | "delete" => format!("kubectl {}", second),
            _ => "kubectl".to_string(),
        },
        "docker" => match second {
            "build" | "ps" | "logs" | "run" | "images" => format!("docker {}", second),
            _ => "docker".to_string(),
        },
        _ => first.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::command_family;

    #[test]
    fn normalizes_git_commands() {
        assert_eq!(command_family("git diff -- src/main.rs"), "git diff");
        assert_eq!(command_family("git status -s"), "git status");
    }

    #[test]
    fn normalizes_cargo_commands() {
        assert_eq!(command_family("cargo build --release"), "cargo build");
        assert_eq!(command_family("cargo test foo"), "cargo test");
    }

    #[test]
    fn falls_back_to_binary() {
        assert_eq!(command_family("python script.py"), "python");
        assert_eq!(command_family(""), "unknown");
    }
}
