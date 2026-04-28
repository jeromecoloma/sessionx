use anyhow::Result;

pub fn run(names_only: bool) -> Result<()> {
    let loaded = crate::config::find_and_load()?;
    let prefix = loaded.session_prefix();
    let sessions = crate::tmux::list_sessions()?;
    let mut found = false;
    for s in sessions {
        if !s.starts_with(&prefix) {
            continue;
        }
        let handle = &s[prefix.len()..];
        if names_only {
            println!("{handle}");
        } else if loaded.worktree_mode() {
            let wt = crate::worktree::worktree_path(&loaded, handle)
                .ok()
                .map(|p| format!("  {}", p.display()))
                .unwrap_or_default();
            println!("{s}{wt}");
        } else {
            println!("{s}");
        }
        found = true;
    }
    if !found && !names_only {
        eprintln!("no sessions matching prefix '{prefix}'");
    }
    Ok(())
}
