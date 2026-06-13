//! `sessionx agent-hooks <install|uninstall|status>` — wire Claude Code hooks.
//!
//! Installs three hooks into `~/.claude/settings.json` that report agent state
//! to the dashboard via `sessionx agent-state`:
//!
//!   UserPromptSubmit → working   (a turn started)
//!   Notification     → blocked   (Claude is waiting on approval/input)
//!   Stop             → done      (the turn finished)
//!
//! Each hook is guarded so it no-ops outside tmux or when sessionx is missing
//! from PATH. Our entries are identified by the `sessionx agent-state`
//! substring, so install is idempotent and uninstall removes exactly what we
//! added — other hooks in the file are preserved untouched.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

/// (Claude Code hook event, sessionx state it reports).
const EVENTS: &[(&str, &str)] = &[
    ("UserPromptSubmit", "working"),
    ("Notification", "blocked"),
    ("Stop", "done"),
];

/// Substring that marks a hook command as ours.
const MARKER: &str = "sessionx agent-state";

fn hook_command(state: &str) -> String {
    format!(
        "command -v sessionx >/dev/null && [ -n \"$TMUX_PANE\" ] && sessionx agent-state {state} || true"
    )
}

fn settings_path() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".claude/settings.json"))
        .ok_or_else(|| anyhow!("cannot resolve home directory"))
}

fn load_settings(path: &PathBuf) -> Result<Map<String, Value>> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let s = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if s.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(&s)? {
        Value::Object(m) => Ok(m),
        _ => Err(anyhow!("{} is not a JSON object", path.display())),
    }
}

fn save_settings(path: &PathBuf, settings: &Map<String, Value>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(&Value::Object(settings.clone()))?;
    std::fs::write(path, body + "\n").with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// True when any matcher-group under `event` contains one of our commands.
fn event_has_marker(hooks: &Map<String, Value>, event: &str) -> bool {
    let Some(Value::Array(groups)) = hooks.get(event) else {
        return false;
    };
    groups.iter().any(group_has_marker)
}

fn group_has_marker(group: &Value) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .map(|entries| {
            entries.iter().any(|e| {
                e.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.contains(MARKER))
            })
        })
        .unwrap_or(false)
}

pub fn run_install() -> Result<()> {
    let path = settings_path()?;
    let mut settings = load_settings(&path)?;
    let hooks = settings
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow!("\"hooks\" in settings.json is not an object"))?;

    let mut added = 0;
    for (event, state) in EVENTS {
        if event_has_marker(hooks, event) {
            continue;
        }
        let group = json!({
            "hooks": [{ "type": "command", "command": hook_command(state) }]
        });
        match hooks
            .entry((*event).to_string())
            .or_insert_with(|| json!([]))
        {
            Value::Array(groups) => groups.push(group),
            other => return Err(anyhow!("hooks.{event} is not an array: {other}")),
        }
        added += 1;
    }

    if added == 0 {
        println!("already installed: {}", path.display());
        return Ok(());
    }
    save_settings(&path, &settings)?;
    println!("installed {added} hook(s) into {}:", path.display());
    for (event, state) in EVENTS {
        println!("  {event:<18} → sessionx agent-state {state}");
    }
    println!("\nrestart running Claude Code sessions to pick them up.");
    Ok(())
}

pub fn run_uninstall() -> Result<()> {
    let path = settings_path()?;
    let mut settings = load_settings(&path)?;
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        println!("nothing to remove: {}", path.display());
        return Ok(());
    };

    let mut removed = 0;
    let mut empty_events = vec![];
    for (event, groups) in hooks.iter_mut() {
        if let Value::Array(arr) = groups {
            let before = arr.len();
            arr.retain(|g| !group_has_marker(g));
            removed += before - arr.len();
            if arr.is_empty() {
                empty_events.push(event.clone());
            }
        }
    }
    for event in empty_events {
        hooks.remove(&event);
    }
    if hooks.is_empty() {
        settings.remove("hooks");
    }

    if removed == 0 {
        println!("nothing to remove: {}", path.display());
        return Ok(());
    }
    save_settings(&path, &settings)?;
    println!("removed {removed} hook(s) from {}", path.display());
    Ok(())
}

pub fn run_status() -> Result<()> {
    let path = settings_path()?;
    let settings = load_settings(&path)?;
    let hooks = settings.get("hooks").and_then(Value::as_object);
    println!("settings: {}", path.display());
    for (event, state) in EVENTS {
        let installed = hooks.is_some_and(|h| event_has_marker(h, event));
        let mark = if installed { "✓" } else { "✗" };
        println!("  {mark} {event:<18} → sessionx agent-state {state}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_detection() {
        let group = json!({
            "hooks": [{ "type": "command", "command": hook_command("working") }]
        });
        assert!(group_has_marker(&group));
        let other = json!({
            "hooks": [{ "type": "command", "command": "echo hi" }]
        });
        assert!(!group_has_marker(&other));
    }

    #[test]
    fn install_shape_roundtrips() {
        // Simulate install into a settings map with a pre-existing foreign hook.
        let mut hooks = Map::new();
        hooks.insert(
            "Stop".into(),
            json!([{ "hooks": [{ "type": "command", "command": "echo done" }] }]),
        );
        assert!(!event_has_marker(&hooks, "Stop"));
        if let Value::Array(arr) = hooks.get_mut("Stop").unwrap() {
            arr.push(json!({
                "hooks": [{ "type": "command", "command": hook_command("done") }]
            }));
        }
        assert!(event_has_marker(&hooks, "Stop"));
        // Foreign hook untouched.
        let arr = hooks.get("Stop").unwrap().as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert!(!group_has_marker(&arr[0]));
    }
}
