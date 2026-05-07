use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::StatusSpec;
use crate::{cmd, config, picker, status, themes, tmux};
use tmux::ManagedSession;

const DEFAULT_HANDLE: &str = "main";
const ROOT_HANDLE: &str = "root";

enum Action {
    Attach,
    AttachRoot,
    AttachWorktree,
    Init,
    Open(String),
    OpenPlain(String),
    OrphanWorktree(String),
    PlainTmux,
    Quit,
}

pub fn run() -> Result<()> {
    if !picker::is_tty() {
        eprintln!("sessionx: no subcommand given. Run `sessionx --help` for usage.");
        std::process::exit(2);
    }

    // Note: we don't bail out when already inside a sessionx-attached session.
    // Switching to another project's session via switch-client is always safe;
    // creation paths (Attach/AttachRoot/AttachWorktree) enforce their own
    // nesting guard inside `cmd::add::run`.

    let loaded = config::find_and_load().ok();
    let cwd = std::env::current_dir()?;
    let in_git = is_git_repo(&cwd);
    let mut managed = tmux::list_managed_sessions().unwrap_or_default();

    // Group: current project's managed sessions first (root before others),
    // then sessions from other projects.
    if let Some(l) = &loaded {
        let here = l.project_root.display().to_string();
        managed.sort_by_key(|m| {
            let local = m.project == here;
            let is_root = m.handle == ROOT_HANDLE;
            match (local, is_root) {
                (true, true) => 0,
                (true, false) => 1,
                (false, _) => 2,
            }
        });
    }

    // Hide the "Attach/create..." entries when the corresponding session
    // already exists for this project — the user can just open it.
    let (root_exists, main_exists) = if let Some(l) = &loaded {
        let here = l.project_root.display().to_string();
        let has = |handle: &str| {
            managed
                .iter()
                .any(|m| m.project == here && m.handle == handle)
        };
        (has(ROOT_HANDLE), has(DEFAULT_HANDLE))
    } else {
        (false, false)
    };

    let mut labels: Vec<String> = vec![];
    let mut actions: Vec<Action> = vec![];

    if let Some(l) = &loaded {
        if l.worktree_mode() {
            if !root_exists {
                labels.push(format!(
                    "Attach/create main project session ({ROOT_HANDLE})"
                ));
                actions.push(Action::AttachRoot);
            }
            labels.push("Add new worktree session…".to_string());
            actions.push(Action::AttachWorktree);
        } else if !main_exists {
            labels.push(format!("Attach/create project session ({DEFAULT_HANDLE})"));
            actions.push(Action::Attach);
        }
    } else if in_git {
        labels.push("Init .sessionx.yaml here".to_string());
        actions.push(Action::Init);
    }

    let here = loaded.as_ref().map(|l| l.project_root.display().to_string());
    for m in &managed {
        let is_local = here.as_deref() == Some(m.project.as_str());
        let label = if is_local {
            format!("Open managed session: {}", m.name)
        } else {
            format!(
                "Open managed session: {}  \x1b[2m[{}]\x1b[0m",
                m.name, m.project
            )
        };
        labels.push(label);
        actions.push(Action::Open(m.name.clone()));
    }

    if let Some(l) = &loaded {
        for handle in orphan_worktrees(l, &managed) {
            labels.push(format!(
                "Clean up orphan worktree: {handle}  \x1b[2m[no session]\x1b[0m"
            ));
            actions.push(Action::OrphanWorktree(handle));
        }
    }

    let unmanaged = tmux::list_unmanaged_sessions().unwrap_or_default();
    for name in &unmanaged {
        labels.push(format!("Attach plain tmux session: {name}"));
        actions.push(Action::OpenPlain(name.clone()));
    }

    labels.push("New plain tmux session".to_string());
    actions.push(Action::PlainTmux);

    labels.push("Quit".to_string());
    actions.push(Action::Quit);

    let (expect_keys, header): (&[&str], Option<&str>) =
        if managed.is_empty() && unmanaged.is_empty() {
            (&[], None)
        } else {
            (
                &["ctrl-x"],
                Some("enter: select  ·  ctrl-x: delete session"),
            )
        };
    let Some((idx, key)) = picker::select_with_keys("sessionx", &labels, expect_keys, header)?
    else {
        return Ok(());
    };

    if key.as_deref() == Some("ctrl-x") {
        match &actions[idx] {
            Action::Open(name) => {
                if let Some(m) = managed.iter().find(|m| &m.name == name) {
                    return delete_managed(m);
                }
            }
            Action::OpenPlain(name) => {
                return delete_plain(name);
            }
            Action::OrphanWorktree(handle) => {
                return delete_orphan_worktree(loaded.as_ref(), handle);
            }
            _ => {}
        }
        eprintln!("sessionx: ctrl-x only deletes existing sessions");
        return Ok(());
    }

    match &actions[idx] {
        Action::Attach => cmd::add::run(DEFAULT_HANDLE, None, true, false),
        Action::AttachRoot => cmd::add::run(ROOT_HANDLE, None, true, true),
        Action::AttachWorktree => {
            let Some(handle) = picker::prompt("worktree handle (e.g. feat-x)")? else {
                return Ok(());
            };
            cmd::add::run(&handle, None, true, false)
        }
        Action::Init => cmd::init::run(cmd::init::InitOpts::default()),
        Action::Open(name) => cmd::open::run(Some(name), false),
        Action::OpenPlain(name) => tmux::attach_or_switch(name),
        Action::OrphanWorktree(handle) => delete_orphan_worktree(loaded.as_ref(), handle),
        Action::PlainTmux => plain_tmux(&cwd),
        Action::Quit => Ok(()),
    }
}

fn orphan_worktrees(loaded: &config::Loaded, managed: &[ManagedSession]) -> Vec<String> {
    if !loaded.worktree_mode() {
        return vec![];
    }
    let Some(dir) = loaded.config.worktree_dir.as_deref() else {
        return vec![];
    };
    let base = std::path::Path::new(dir);
    let abs = if base.is_absolute() {
        base.to_path_buf()
    } else {
        loaded.project_root.join(base)
    };
    let Ok(entries) = std::fs::read_dir(&abs) else {
        return vec![];
    };
    let here = loaded.project_root.display().to_string();
    let mut active: std::collections::HashSet<String> = std::collections::HashSet::new();
    for m in managed {
        if m.project == here {
            active.insert(m.handle.clone());
        }
    }
    let mut out = vec![];
    for e in entries.flatten() {
        if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Some(name) = e.file_name().to_str().map(|s| s.to_string()) else {
            continue;
        };
        if !active.contains(&name) {
            out.push(name);
        }
    }
    out.sort();
    out
}

fn delete_orphan_worktree(loaded: Option<&config::Loaded>, handle: &str) -> Result<()> {
    let Some(loaded) = loaded else {
        eprintln!("sessionx: no project config in cwd — cannot clean orphan");
        return Ok(());
    };
    let msg = format!("Force-remove orphan worktree '{handle}'?");
    if !picker::confirm(&msg, false)? {
        return Ok(());
    }
    cmd::rm::run_with_loaded(loaded, handle, true)
}

fn delete_plain(name: &str) -> Result<()> {
    let msg = format!("Kill plain tmux session '{name}'?");
    if !picker::confirm(&msg, false)? {
        return Ok(());
    }
    if tmux::has_session(name) {
        tmux::kill_session(name)?;
        println!("killed session {name}");
    }
    Ok(())
}

fn delete_managed(m: &ManagedSession) -> Result<()> {
    let msg = format!("Delete session '{}' ({})?", m.name, m.project);
    if !picker::confirm(&msg, false)? {
        return Ok(());
    }
    let project_path = Path::new(&m.project);
    let handle = if m.handle.is_empty() {
        DEFAULT_HANDLE
    } else {
        &m.handle
    };
    match config::load_from_dir(project_path) {
        Ok(loaded) => cmd::rm::run_with_loaded(&loaded, handle, false),
        Err(_) => {
            if tmux::has_session(&m.name) {
                tmux::kill_session(&m.name)?;
                println!("killed session {}", m.name);
            }
            eprintln!(
                "sessionx: project config not found at {} — worktree cleanup skipped",
                m.project
            );
            Ok(())
        }
    }
}

fn is_git_repo(cwd: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn plain_tmux(cwd: &Path) -> Result<()> {
    let Some(theme) = pick_plain_theme()? else {
        return Ok(());
    };

    let base = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("tmux");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut name = sanitize(&format!("tmux-{base}-{}", stamp % 100000));
    let mut suffix = 0u32;
    while tmux::has_session(&name) {
        suffix += 1;
        name = sanitize(&format!("tmux-{base}-{}-{suffix}", stamp % 100000));
    }

    name = picker::maybe_rename_long(name, 20, sanitize, tmux::has_session)?;

    tmux::new_session(&name, cwd, None)?;

    if let Some(theme_name) = theme {
        let spec = StatusSpec {
            enabled: true,
            theme: Some(theme_name),
            ..StatusSpec::default()
        };
        status::apply(&name, &spec)?;
    }

    tmux::attach_or_switch(&name)
}

/// Prompt for a theme to apply to a plain tmux session.
/// Outer `Option` distinguishes cancel (`None` → abort spawn) from a real selection.
/// Inner `Option<String>` carries the theme name, or `None` for the "(no theme)" entry.
fn pick_plain_theme() -> Result<Option<Option<String>>> {
    let none_label = "(no theme)".to_string();
    let mut items = vec![none_label];
    items.extend(themes::list().iter().map(|s| s.to_string()));

    let Some(idx) = picker::select("theme for plain tmux session", &items)? else {
        return Ok(None);
    };
    if idx == 0 {
        Ok(Some(None))
    } else {
        Ok(Some(Some(items[idx].clone())))
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if matches!(c, '.' | ':' | ' ' | '\t') {
                '_'
            } else {
                c
            }
        })
        .collect()
}
