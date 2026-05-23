use anyhow::Result;
use std::io::IsTerminal;

use anstyle::{AnsiColor, Effects, Style};

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
        let project_root = crate::config::find_and_load()
            .ok()
            .map(|loaded| loaded.project_root.display().to_string());
        if let Some(root) = project_root.as_deref() {
            managed.sort_by_key(|m| if m.project == root { 0 } else { 1 });
        }

        if names_only {
            for m in &managed {
                println!("{}", m.name);
            }
            return Ok(());
        }

        let pretty = std::io::stdout().is_terminal();
        if !pretty {
            for m in &managed {
                println!("{}\t{}\t{}", m.name, m.handle, m.project);
            }
            return Ok(());
        }

        print_pretty(&managed, project_root.as_deref());
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

fn print_pretty(managed: &[crate::tmux::ManagedSession], project_root: Option<&str>) {
    let use_color = std::env::var_os("NO_COLOR").is_none();
    let home = dirs::home_dir().map(|p| p.display().to_string());

    let display_path = |p: &str| -> String {
        match home.as_deref() {
            Some(h) if !h.is_empty() && (p == h || p.starts_with(&format!("{h}/"))) => {
                format!("~{}", &p[h.len()..])
            }
            _ => p.to_string(),
        }
    };

    let name_w = managed
        .iter()
        .map(|m| m.name.len())
        .chain(std::iter::once("SESSION".len()))
        .max()
        .unwrap_or(0);
    let branch_w = managed
        .iter()
        .map(|m| m.handle.len())
        .chain(std::iter::once("BRANCH".len()))
        .max()
        .unwrap_or(0);

    let session_style = Style::new()
        .fg_color(Some(AnsiColor::Cyan.into()))
        .effects(Effects::BOLD);
    let branch_style = Style::new().fg_color(Some(AnsiColor::Yellow.into()));
    let path_style = Style::new().effects(Effects::DIMMED);
    let header_style = Style::new().effects(Effects::DIMMED | Effects::BOLD);

    let paint = |s: &str, style: Style| -> String {
        if use_color {
            format!("{style}{s}{style:#}")
        } else {
            s.to_string()
        }
    };

    println!(
        "{}  {}  {}",
        paint(&format!("{:<name_w$}", "SESSION"), header_style),
        paint(&format!("{:<branch_w$}", "BRANCH"), header_style),
        paint("PATH", header_style),
    );

    // Split point: where the sort boundary lands (project sessions vs. others).
    let split = project_root
        .map(|root| managed.iter().take_while(|m| m.project == root).count())
        .unwrap_or(0);
    let show_divider = split > 0 && split < managed.len();

    for (i, m) in managed.iter().enumerate() {
        if show_divider && i == split {
            println!();
        }
        let path = display_path(&m.project);
        println!(
            "{}  {}  {}",
            paint(&format!("{:<name_w$}", m.name), session_style),
            paint(&format!("{:<branch_w$}", m.handle), branch_style),
            paint(&path, path_style),
        );
    }
}
