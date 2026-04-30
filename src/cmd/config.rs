use anyhow::{anyhow, Context, Result};

use crate::agent;

/// `sessionx config` (no args) — open `~/.config/sessionx/config.yaml` in
/// `$VISUAL`/`$EDITOR`. Creates the file with a starter template if absent.
pub fn run_edit() -> Result<()> {
    agent::migrate_legacy_if_present();
    let path = agent::config_path().ok_or_else(|| anyhow!("no config dir"))?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(
            &path,
            "# sessionx global config\n\
             # Set your default AI agent for the `<agent>` placeholder in pane commands.\n\
             # Examples: claude, codex, aider, gh copilot, or any custom command.\n\
             # Override per-session via SX_AGENT=… sessionx add <handle>.\n\n\
             # agent: claude\n",
        )
        .with_context(|| format!("creating {}", path.display()))?;
    }
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());
    let status = std::process::Command::new(&editor).arg(&path).status()?;
    if !status.success() {
        return Err(anyhow!("editor {editor} exited non-zero"));
    }
    Ok(())
}

pub fn run_path() -> Result<()> {
    agent::migrate_legacy_if_present();
    let path = agent::config_path().ok_or_else(|| anyhow!("no config dir"))?;
    println!("{}", path.display());
    Ok(())
}

pub fn run_get(key: Option<&str>) -> Result<()> {
    agent::migrate_legacy_if_present();
    let path = agent::config_path().ok_or_else(|| anyhow!("no config dir"))?;
    if !path.exists() {
        eprintln!("(config not yet created — run `sessionx config` to edit)");
        return Ok(());
    }
    let body = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    match key {
        None => {
            print!("{body}");
            if !body.ends_with('\n') {
                println!();
            }
        }
        Some("agent") => {
            println!("{}", agent::resolve());
        }
        Some(other) => {
            return Err(anyhow!(
                "unknown key '{other}' (try 'agent' or no arg to dump the file)"
            ));
        }
    }
    Ok(())
}

pub fn run_set(key: &str, value: &str) -> Result<()> {
    agent::migrate_legacy_if_present();
    match key {
        "agent" => {
            agent::save_global_agent(value)?;
            let path = agent::config_path().ok_or_else(|| anyhow!("no config dir"))?;
            println!("set agent={value} in {}", path.display());
            Ok(())
        }
        other => Err(anyhow!("unknown key '{other}' (currently only 'agent')")),
    }
}
