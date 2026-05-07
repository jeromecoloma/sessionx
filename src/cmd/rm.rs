use anyhow::Result;
use std::path::Path;

use crate::{config, hooks, tmux, worktree};

/// Accepts either a project handle (resolved against the current project) or
/// the literal name of a managed tmux session — useful for renamed sessions
/// where `prefix+handle` no longer matches the actual session name.
pub fn run(arg: &str, force: bool) -> Result<()> {
    if let Some(m) = tmux::list_managed_sessions()
        .unwrap_or_default()
        .into_iter()
        .find(|m| m.name == arg)
    {
        let handle = if m.handle.is_empty() {
            arg.to_string()
        } else {
            m.handle.clone()
        };
        match config::load_from_dir(Path::new(&m.project)) {
            Ok(loaded) => {
                return run_inner(&loaded, &handle, Some(&m.name), force);
            }
            Err(_) => {
                if tmux::has_session(&m.name) {
                    tmux::kill_session(&m.name)?;
                    println!("killed session {}", m.name);
                }
                eprintln!(
                    "sessionx: project config not found at {} — worktree cleanup skipped",
                    m.project
                );
                return Ok(());
            }
        }
    }
    let loaded = config::find_and_load()?;
    // Recover from a partial teardown: if a previous `sxk` killed the tmux
    // session but failed to remove the worktree (e.g. untracked files without
    // --force), the managed-session record is gone. The user may now be
    // passing either the handle or the full session name. Probe both against
    // worktree paths on disk so `sxk <name> --force` can finish the job.
    let handle = resolve_orphan_handle(&loaded, arg).unwrap_or_else(|| arg.to_string());
    run_with_loaded(&loaded, &handle, force)
}

fn resolve_orphan_handle(loaded: &config::Loaded, arg: &str) -> Option<String> {
    if !loaded.worktree_mode() {
        return None;
    }
    if let Ok(p) = worktree::worktree_path(loaded, arg) {
        if p.exists() {
            return Some(arg.to_string());
        }
    }
    let prefix = loaded.session_prefix();
    if let Some(stripped) = arg.strip_prefix(&prefix) {
        if let Ok(p) = worktree::worktree_path(loaded, stripped) {
            if p.exists() {
                return Some(stripped.to_string());
            }
        }
    }
    None
}

pub fn run_with_loaded(loaded: &config::Loaded, handle: &str, force: bool) -> Result<()> {
    run_inner(loaded, handle, None, force)
}

fn run_inner(
    loaded: &config::Loaded,
    handle: &str,
    session_override: Option<&str>,
    force: bool,
) -> Result<()> {
    let session = session_override.map(str::to_string).unwrap_or_else(|| {
        // The session may have been renamed at creation time (see
        // picker::maybe_rename_long in `add`), so prefix+handle isn't reliable.
        // Prefer the actual session tagged with this handle for our project.
        let project = loaded.project_root.display().to_string();
        tmux::list_managed_sessions()
            .unwrap_or_default()
            .into_iter()
            .find(|m| m.handle == handle && m.project == project)
            .map(|m| m.name)
            .unwrap_or_else(|| loaded.session_name(handle))
    });

    let worktree_path = if loaded.worktree_mode() {
        worktree::worktree_path(loaded, handle).ok()
    } else {
        None
    };
    // Treat as a root-mode teardown when worktree-mode is on but no worktree
    // exists on disk for this handle (e.g. the "root" main-project session).
    let has_worktree_on_disk = worktree_path.as_ref().is_some_and(|p| p.exists());
    let is_root_session = loaded.worktree_mode() && !has_worktree_on_disk;
    let branch = if loaded.worktree_mode() && !is_root_session {
        Some(worktree::handle_to_branch(
            handle,
            loaded.config.worktree_naming,
        ))
    } else {
        None
    };

    // pre_remove hooks run from the worktree (or project root in plain mode).
    // Skip them entirely for root sessions — they're worktree-specific.
    if !is_root_session && !loaded.config.pre_remove.is_empty() {
        let hook_cwd = worktree_path
            .clone()
            .filter(|p| p.exists())
            .unwrap_or_else(|| loaded.project_root.clone());
        let env = hooks::HookEnv {
            vars: hooks::base_env(
                &loaded.project_root,
                handle,
                &session,
                worktree_path.as_deref(),
                branch.as_deref(),
            ),
            cwd: hook_cwd,
        };
        hooks::run_all("pre_remove", &loaded.config.pre_remove, &env)?;
    }

    let killed = if tmux::has_session(&session) {
        tmux::kill_session(&session)?;
        println!("killed session {session}");
        true
    } else {
        false
    };

    let removed_worktree = if loaded.worktree_mode() && !is_root_session {
        worktree::remove(loaded, handle, force)?;
        if let Some(p) = &worktree_path {
            println!("removed worktree {}", p.display());
            true
        } else {
            false
        }
    } else {
        false
    };

    if !killed && !removed_worktree {
        eprintln!("sessionx: no tmux session named '{session}' — nothing to do");
    }
    Ok(())
}
