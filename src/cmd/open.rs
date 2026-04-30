use anyhow::{anyhow, Result};

use crate::tmux;

pub fn run(name: Option<&str>, names_only: bool) -> Result<()> {
    let managed = tmux::list_managed_sessions()?;

    let Some(name) = name else {
        for m in &managed {
            if names_only {
                println!("{}", m.name);
            } else {
                println!("{}\t{}\t{}", m.name, m.handle, m.project);
            }
        }
        return Ok(());
    };

    if managed.iter().any(|m| m.name == name) {
        return tmux::attach_or_switch(name);
    }

    let candidates: Vec<&str> = managed.iter().map(|m| m.name.as_str()).collect();
    Err(anyhow!(
        "no managed session named '{name}'. Candidates: {}",
        if candidates.is_empty() {
            "(none)".to_string()
        } else {
            candidates.join(", ")
        }
    ))
}
