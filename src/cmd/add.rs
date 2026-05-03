use anyhow::{anyhow, Result};
use std::path::Path;

use crate::config::{self, Config, Loaded, PaneSpec, SplitDir};
use crate::{agent, hooks, picker, status, tmux, worktree};

pub fn run(handle: &str, base: Option<&str>, attach: bool) -> Result<()> {
    if tmux::in_sessionx() && std::env::var("SESSIONX_ALLOW_NESTED").is_err() {
        return Err(anyhow!(
            "already running inside a sessionx-attached tmux session; \
             set SESSIONX_ALLOW_NESTED=1 to override"
        ));
    }

    let loaded = config::find_and_load()?;
    let auto_session = loaded.session_name(handle);

    if tmux::has_session(&auto_session) {
        eprintln!("session {auto_session} already exists — attaching");
        if attach {
            tmux::attach_or_switch(&auto_session)?;
        }
        return Ok(());
    }

    let session = picker::maybe_rename_long(
        auto_session,
        20,
        config::sanitize_session,
        tmux::has_session,
    )?;

    // 1. Worktree (if applicable)
    let (work_cwd, branch) = if loaded.worktree_mode() {
        if !worktree::is_git_repo(&loaded.project_root) {
            return Err(anyhow!(
                "worktree_dir set but {} is not a git repository",
                loaded.project_root.display()
            ));
        }
        let p = worktree::create(&loaded, handle, base)?;
        let b = worktree::handle_to_branch(handle, loaded.config.worktree_naming);
        (p, Some(b))
    } else {
        (loaded.project_root.clone(), None)
    };

    // 2. post_create hooks run inside the worktree (or project root) BEFORE tmux,
    //    so when we attach, the env is ready.
    let mut env_vars = hooks::base_env(
        &loaded.project_root,
        handle,
        &session,
        if loaded.worktree_mode() {
            Some(&work_cwd)
        } else {
            None
        },
        branch.as_deref(),
    );
    env_vars.extend(status::icon_env(&loaded.config.status));
    let hook_env = hooks::HookEnv {
        vars: env_vars,
        cwd: work_cwd.clone(),
    };
    if !loaded.config.post_create.is_empty() {
        hooks::run_all("post_create", &loaded.config.post_create, &hook_env)?;
    }

    // 3. Build the tmux session.
    build_session(&loaded, &session, &work_cwd)?;

    // 4. Tag the session so `sessionx open` / `ls --all` can find it cross-project.
    tmux::set_user_option(&session, "sessionx-managed", "1")?;
    tmux::set_user_option(
        &session,
        "sessionx-project",
        &loaded.project_root.display().to_string(),
    )?;
    tmux::set_user_option(&session, "sessionx-handle", handle)?;

    // 5. Status bar (per-session, scoped).
    status::apply(&session, &loaded.config.status)?;

    // 6. Attach (or print).
    if attach {
        tmux::attach_or_switch(&session)?;
    } else {
        println!("{session}");
    }
    Ok(())
}

fn build_session(loaded: &Loaded, session: &str, cwd: &Path) -> Result<()> {
    let cfg: &Config = &loaded.config;

    if let Some(windows) = &cfg.windows {
        let mut iter = windows.iter();
        let first = iter.next().ok_or_else(|| anyhow!("'windows' is empty"))?;
        let (first_wid, first_pid) = tmux::new_session(session, cwd, first.name.as_deref())?;
        let mut first_window_id = first_wid.clone();
        build_panes(&first_wid, &first_pid, cwd, &first.panes)?;

        for w in iter {
            let (wid, pid) = tmux::new_window(session, w.name.as_deref(), cwd)?;
            build_panes(&wid, &pid, cwd, &w.panes)?;
        }

        // Select the first window after building all of them.
        tmux::select_window(&first_window_id)?;
        // Touch to silence unused-mut warning in some configs.
        first_window_id.clear();
    } else if let Some(panes) = &cfg.panes {
        let (wid, pid) = tmux::new_session(session, cwd, None)?;
        build_panes(&wid, &pid, cwd, panes)?;
    } else {
        tmux::new_session(session, cwd, None)?;
    }
    Ok(())
}

/// Send commands into the first pane (already created with the window) and
/// split for additional panes, capturing each new pane_id.
fn build_panes(window_id: &str, first_pane_id: &str, cwd: &Path, panes: &[PaneSpec]) -> Result<()> {
    if panes.is_empty() {
        return Ok(());
    }
    let mut pane_ids: Vec<String> = vec![first_pane_id.to_string()];
    if let Some(cmd) = panes[0].command.as_deref().filter(|s| !s.is_empty()) {
        tmux::send_keys(&pane_ids[0], &agent::expand(cmd))?;
    }
    for p in panes.iter().skip(1) {
        let horizontal = matches!(p.split, Some(SplitDir::Horizontal));
        // Split off the most recently created pane (or the window if first split).
        let target = pane_ids
            .last()
            .cloned()
            .unwrap_or_else(|| window_id.to_string());
        let new_pid = tmux::split_window(&target, cwd, horizontal, p.percentage, p.size)?;
        if let Some(cmd) = p.command.as_deref().filter(|s| !s.is_empty()) {
            tmux::send_keys(&new_pid, &agent::expand(cmd))?;
        }
        pane_ids.push(new_pid);
    }
    if let Some(focus_idx) = panes.iter().position(|p| p.focus) {
        tmux::select_pane(&pane_ids[focus_idx])?;
    }
    Ok(())
}
