use anyhow::{anyhow, Context, Result};

use crate::config::{self, StatusSpec};
use crate::status;
use crate::themes;
use crate::tmux;

/// `sessionx theme` — list available themes and mark current one if a project is in scope.
pub fn run_list() -> Result<()> {
    let current = config::find_and_load()
        .ok()
        .and_then(|l| l.config.status.theme.clone());

    if let Some(t) = &current {
        println!("current: {t}");
    } else {
        println!("current: (none)");
    }
    println!("available:");
    for name in themes::list() {
        let marker = match &current {
            Some(c) if c == name => " *",
            _ => "",
        };
        println!("  {name}{marker}");
    }
    Ok(())
}

/// `sessionx theme set <name>` — write theme to .sessionx.yaml; live-apply unless --no-apply.
pub fn run_set(name: &str, apply: bool, session: Option<&str>) -> Result<()> {
    themes::load(name)?; // validate
    let loaded = config::find_and_load()
        .context("set requires a .sessionx.yaml in cwd or a parent directory")?;

    config::set_theme_in_file(&loaded.config_path, name)?;
    println!("wrote theme '{name}' to {}", loaded.config_path.display());

    if !apply {
        return Ok(());
    }

    let target = resolve_session(session)?;
    let Some(target) = target else {
        eprintln!("not inside tmux — theme will apply on next `sessionx add`");
        return Ok(());
    };

    let mut spec: StatusSpec = loaded.config.status.clone();
    spec.theme = Some(name.to_string());
    status::apply(&target, &spec)?;
    println!("applied to session '{target}'");
    Ok(())
}

/// `sessionx theme preview <name>` — apply to a running session without touching YAML.
pub fn run_preview(name: &str, session: Option<&str>) -> Result<()> {
    themes::load(name)?; // validate

    let target = resolve_session(session)?
        .ok_or_else(|| anyhow!("not inside tmux — pass --session <name>"))?;

    // Use the project's status spec as a base if we can find one, else default.
    let mut spec: StatusSpec = config::find_and_load()
        .map(|l| l.config.status.clone())
        .unwrap_or_default();
    spec.theme = Some(name.to_string());
    status::apply(&target, &spec)?;
    println!("preview '{name}' applied to session '{target}' (yaml unchanged)");
    Ok(())
}

fn resolve_session(explicit: Option<&str>) -> Result<Option<String>> {
    if let Some(s) = explicit {
        return Ok(Some(s.to_string()));
    }
    tmux::current_session()
}
