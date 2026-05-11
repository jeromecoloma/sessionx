use anyhow::Result;

pub fn run(names_only: bool, all: bool) -> Result<()> {
    if all {
        let mut managed = crate::tmux::list_managed_sessions()?;
        if managed.is_empty() {
            if !names_only {
                eprintln!("no managed sessionx sessions");
            }
            return Ok(());
        }
        // If invoked inside a project (.sessionx.yaml found walking up), sort
        // sessions belonging to that project to the top so picker helpers like
        // `sxa` surface related sessions first.
        if let Ok(loaded) = crate::config::find_and_load() {
            let project_root = loaded.project_root.display().to_string();
            managed.sort_by_key(|m| if m.project == project_root { 0 } else { 1 });
        }
        for m in &managed {
            if names_only {
                println!("{}", m.name);
            } else {
                println!("{}\t{}\t{}", m.name, m.handle, m.project);
            }
        }
        return Ok(());
    }

    let loaded = crate::config::find_and_load()?;
    let prefix = loaded.session_prefix();
    let project_root = loaded.project_root.display().to_string();
    let managed = crate::tmux::list_managed_sessions().unwrap_or_default();
    let sessions = crate::tmux::list_sessions()?;

    let mut emitted: std::collections::BTreeSet<String> = Default::default();
    let mut entries: Vec<(String, String)> = vec![]; // (session_name, handle)

    // 1. Managed sessions tagged with this project root (handles renamed sessions).
    for m in &managed {
        if m.project == project_root && emitted.insert(m.name.clone()) {
            entries.push((m.name.clone(), m.handle.clone()));
        }
    }

    // 2. Legacy / untagged sessions matched by prefix.
    for s in &sessions {
        if !s.starts_with(&prefix) {
            continue;
        }
        if !emitted.insert(s.clone()) {
            continue;
        }
        let handle = s[prefix.len()..].to_string();
        entries.push((s.clone(), handle));
    }

    let found = !entries.is_empty();
    for (name, handle) in &entries {
        if names_only {
            println!("{handle}");
        } else if loaded.worktree_mode() {
            let wt = crate::worktree::worktree_path(&loaded, handle)
                .ok()
                .map(|p| format!("  {}", p.display()))
                .unwrap_or_default();
            println!("{name}{wt}");
        } else {
            println!("{name}");
        }
    }
    if !found && !names_only {
        eprintln!("no sessions for project '{project_root}'");
    }
    Ok(())
}
