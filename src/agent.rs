//! Agent command resolution.
//!
//! Pane commands may use `<agent>` as a placeholder for the user's preferred
//! AI CLI (claude / codex / aider / etc.). The actual command is resolved at
//! session-build time from, in order:
//!
//! 1. `SX_AGENT` environment variable.
//! 2. `agent:` field in `~/.config/sessionx/config.yaml`.
//! 3. Built-in default: `"claude"`.
//!
//! Lets users pick their agent globally without editing every project's
//! `.sessionx.yaml`.

use serde::Deserialize;
use std::path::PathBuf;

// When no agent is configured (no SX_AGENT, no ~/.config/sessionx/config.yaml
// `agent:` field), `<agent>` substitutes to a plain shell rather than picking
// an AI CLI on the user's behalf. They can run their preferred agent manually,
// set SX_AGENT, or run `sessionx init --force` to choose one.
const DEFAULT_AGENT: &str = "exec $SHELL";

#[derive(Debug, Deserialize, Default)]
struct GlobalConfig {
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    git_main_branches: Option<Vec<String>>,
}

/// Path to the global config. Always uses XDG semantics
/// (`$XDG_CONFIG_HOME/sessionx/config.yaml`, default `~/.config/sessionx/config.yaml`)
/// — even on macOS, where `dirs::config_dir()` would otherwise pick
/// `~/Library/Application Support/`. Most CLI tools live under `~/.config`,
/// matching how this is documented in the README.
pub fn config_path() -> Option<PathBuf> {
    let base = match std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(s) => PathBuf::from(s),
        None => dirs::home_dir()?.join(".config"),
    };
    Some(base.join("sessionx/config.yaml"))
}

/// Legacy macOS location used briefly while config_path() relied on
/// `dirs::config_dir()`. We auto-migrate it on first read.
fn legacy_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("Library/Application Support/sessionx/config.yaml"))
}

/// Move a legacy `~/Library/Application Support/sessionx/config.yaml` (if any)
/// to the canonical `~/.config/sessionx/config.yaml`. No-op on non-macOS or
/// when the legacy file doesn't exist. Idempotent. Public so cmd::config can
/// trigger it explicitly when the user is touching the config directly.
pub fn migrate_legacy_if_present() {
    let Some(canonical) = config_path() else {
        return;
    };
    if canonical.exists() {
        return;
    }
    let Some(legacy) = legacy_config_path() else {
        return;
    };
    if !legacy.exists() {
        return;
    }
    if let Some(parent) = canonical.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let _ = std::fs::rename(&legacy, &canonical);
    // Best-effort cleanup of the now-empty legacy dir.
    if let Some(p) = legacy.parent() {
        let _ = std::fs::remove_dir(p);
    }
}

fn load_global() -> GlobalConfig {
    migrate_legacy_if_present();
    let Some(p) = config_path() else {
        return GlobalConfig::default();
    };
    let Ok(s) = std::fs::read_to_string(&p) else {
        return GlobalConfig::default();
    };
    serde_yaml::from_str(&s).unwrap_or_default()
}

/// Branches that the status bar's git segment treats as "main-line"
/// (rendered with the git-logo icon). Defaults to `["main", "master"]`
/// when the global config is missing or has no override.
pub fn global_git_main_branches() -> Vec<String> {
    let cfg = load_global();
    cfg.git_main_branches
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec!["main".to_string(), "master".to_string()])
}

/// True when `~/.config/sessionx/config.yaml` exists and already has a non-empty `agent:` field.
pub fn global_agent_set() -> bool {
    load_global()
        .agent
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// Persist `agent: <value>` to the global config, creating the file if needed.
/// Preserves any other top-level keys already present.
pub fn save_global_agent(value: &str) -> anyhow::Result<()> {
    use anyhow::Context;
    let path = config_path().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = rewrite_agent(&existing, value);
    std::fs::write(&path, updated).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Inline rewrite: replace the first top-level `agent:` line, or append one
/// if missing. Keeps comments and other keys intact.
fn rewrite_agent(input: &str, value: &str) -> String {
    let mut out = String::with_capacity(input.len() + 32);
    let mut wrote = false;
    let trailing_nl = input.ends_with('\n') || input.is_empty();
    let line_count = input.lines().count();
    for (i, line) in input.lines().enumerate() {
        let last = i + 1 == line_count;
        if !wrote
            && line.trim_start().starts_with("agent:")
            && !line.starts_with(' ')
            && !line.starts_with('\t')
        {
            out.push_str(&format!("agent: {value}"));
            wrote = true;
        } else {
            out.push_str(line);
        }
        if !last || trailing_nl {
            out.push('\n');
        }
    }
    if !wrote {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("agent: {value}\n"));
    }
    out
}

/// Resolve the agent command. Trims surrounding whitespace; falls back to the
/// default when nothing usable is configured.
pub fn resolve() -> String {
    if let Ok(v) = std::env::var("SX_AGENT") {
        let t = v.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    if let Some(v) = load_global().agent {
        let t = v.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    DEFAULT_AGENT.to_string()
}

/// Replace every `<agent>` substring in `cmd` with the resolved agent command.
pub fn expand(cmd: &str) -> String {
    if !cmd.contains("<agent>") {
        return cmd.to_string();
    }
    cmd.replace("<agent>", &resolve())
}

#[cfg(test)]
mod tests {
    // Single test — env vars are process-wide, so Rust's parallel test
    // runner would race separate tests against each other.
    use super::*;

    #[test]
    fn rewrite_agent_replaces_existing() {
        let input = "# header\nagent: claude\nfoo: bar\n";
        let out = rewrite_agent(input, "codex");
        assert_eq!(out, "# header\nagent: codex\nfoo: bar\n");
    }

    #[test]
    fn rewrite_agent_appends_when_missing() {
        let input = "# header\nfoo: bar\n";
        let out = rewrite_agent(input, "aider");
        assert_eq!(out, "# header\nfoo: bar\nagent: aider\n");
    }

    #[test]
    fn rewrite_agent_creates_when_empty() {
        let out = rewrite_agent("", "claude");
        assert_eq!(out, "agent: claude\n");
    }

    #[test]
    fn agent_resolution_and_expansion() {
        // No placeholder → unchanged regardless of agent value.
        std::env::set_var("SX_AGENT", "claude");
        assert_eq!(expand("exec $SHELL"), "exec $SHELL");

        // Placeholder substitutes from env override.
        std::env::set_var("SX_AGENT", "codex");
        assert_eq!(expand("<agent>"), "codex");
        assert_eq!(expand("cd www && <agent>"), "cd www && codex");
        assert_eq!(resolve(), "codex");

        // Different env value → different resolution.
        std::env::set_var("SX_AGENT", "aider");
        assert_eq!(resolve(), "aider");

        // Blank env var doesn't propagate; resolve falls back to config or default.
        std::env::set_var("SX_AGENT", "   ");
        assert!(!resolve().trim().is_empty());

        std::env::remove_var("SX_AGENT");
    }
}
