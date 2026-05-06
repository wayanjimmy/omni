#[derive(Debug, Clone, Copy)]
pub enum ContentHint {
    Code,
    Prose,
    Json,
    BuildLog,
    Mixed,
}

pub fn detect_content_hint(tool_name: &str, command_or_path: &str) -> ContentHint {
    let lower = command_or_path.to_lowercase();
    match tool_name {
        "Read" | "Edit" | "Write" | "Create" | "MultiEdit" => {
            if lower.ends_with(".json")
                || lower.ends_with(".toml")
                || lower.ends_with(".yaml")
                || lower.ends_with(".yml")
            {
                ContentHint::Json
            } else if lower.ends_with(".rs")
                || lower.ends_with(".py")
                || lower.ends_with(".ts")
                || lower.ends_with(".js")
                || lower.ends_with(".go")
                || lower.ends_with(".c")
                || lower.ends_with(".cpp")
                || lower.ends_with(".h")
                || lower.ends_with(".java")
                || lower.ends_with(".cs")
            {
                ContentHint::Code
            } else if lower.ends_with(".md") || lower.ends_with(".txt") {
                ContentHint::Prose
            } else {
                ContentHint::Mixed
            }
        }
        "Bash" => {
            if lower.contains("build")
                || lower.contains("make")
                || lower.contains("cargo")
                || lower.contains("npm install")
            {
                ContentHint::BuildLog
            } else if lower.contains("cat ")
                && (lower.ends_with(".json")
                    || lower.ends_with(".toml")
                    || lower.ends_with(".yaml"))
            {
                ContentHint::Json
            } else if lower.contains("cat ")
                && (lower.ends_with(".rs") || lower.ends_with(".py") || lower.ends_with(".ts"))
            {
                ContentHint::Code
            } else if lower.contains("cat ") && (lower.ends_with(".md") || lower.ends_with(".txt"))
            {
                ContentHint::Prose
            } else {
                ContentHint::Mixed
            }
        }
        _ => ContentHint::Mixed,
    }
}

pub fn estimate_tokens(bytes: usize, hint: ContentHint) -> usize {
    let chars_per_token = match hint {
        ContentHint::Code => 3.2,
        ContentHint::Prose => 4.5,
        ContentHint::Json => 2.8,
        ContentHint::BuildLog => 3.8,
        ContentHint::Mixed => 3.8,
    };
    (bytes as f64 / chars_per_token).ceil() as usize
}
