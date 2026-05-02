use anyhow::Result;

use crate::{config, hooks, tmux, worktree};

pub fn run(handle: &str, force: bool) -> Result<()> {
    let loaded = config::find_and_load()?;
    run_with_loaded(&loaded, handle, force)
}

pub fn run_with_loaded(loaded: &config::Loaded, handle: &str, force: bool) -> Result<()> {
    let session = loaded.session_name(handle);

    let worktree_path = if loaded.worktree_mode() {
        worktree::worktree_path(loaded, handle).ok()
    } else {
        None
    };
    let branch = if loaded.worktree_mode() {
        Some(worktree::handle_to_branch(
            handle,
            loaded.config.worktree_naming,
        ))
    } else {
        None
    };

    // pre_remove hooks run from the worktree (or project root in plain mode).
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
    if !loaded.config.pre_remove.is_empty() {
        hooks::run_all("pre_remove", &loaded.config.pre_remove, &env)?;
    }

    if tmux::has_session(&session) {
        tmux::kill_session(&session)?;
        println!("killed session {session}");
    }

    if loaded.worktree_mode() {
        worktree::remove(loaded, handle, force)?;
        if let Some(p) = &worktree_path {
            println!("removed worktree {}", p.display());
        }
    }
    Ok(())
}
