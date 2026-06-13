use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::{Command, Stdio};

fn verbose() -> bool {
    std::env::var("SX_VERBOSE").ok().as_deref() == Some("1")
}

fn run(args: &[&str]) -> Result<String> {
    if verbose() {
        eprintln!("+ tmux {}", args.join(" "));
    }
    let out = Command::new("tmux").args(args).output()?;
    if !out.status.success() {
        return Err(anyhow!(
            "tmux {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_quiet(args: &[&str]) -> bool {
    if verbose() {
        eprintln!("+ tmux {}", args.join(" "));
    }
    Command::new("tmux")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn has_session(name: &str) -> bool {
    run_quiet(&["has-session", "-t", &format!("={}", name)])
}

/// Returns the current tmux client's session name when invoked from inside tmux.
pub fn current_session() -> Result<Option<String>> {
    if std::env::var("TMUX").is_err() {
        return Ok(None);
    }
    let out = run(&["display-message", "-p", "#S"])?;
    let s = out.trim().to_string();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

/// Returns `(window_id, pane_id)` for the session's first window/pane.
pub fn new_session(
    name: &str,
    cwd: &Path,
    first_window_name: Option<&str>,
) -> Result<(String, String)> {
    let cwd_s = cwd.to_string_lossy().to_string();
    let mut args = vec![
        "new-session",
        "-d",
        "-s",
        name,
        "-c",
        cwd_s.as_str(),
        "-P",
        "-F",
        "#{window_id} #{pane_id}",
    ];
    if let Some(w) = first_window_name {
        args.push("-n");
        args.push(w);
    }
    let out = run(&args)?;
    parse_two(&out)
}

/// Returns `(window_id, pane_id)` for the new window's first pane.
pub fn new_window(session: &str, name: Option<&str>, cwd: &Path) -> Result<(String, String)> {
    let cwd_s = cwd.to_string_lossy().to_string();
    let mut args = vec![
        "new-window",
        "-t",
        session,
        "-c",
        cwd_s.as_str(),
        "-P",
        "-F",
        "#{window_id} #{pane_id}",
    ];
    if let Some(n) = name {
        args.push("-n");
        args.push(n);
    }
    let out = run(&args)?;
    parse_two(&out)
}

fn parse_two(out: &str) -> Result<(String, String)> {
    let line = out
        .lines()
        .next()
        .ok_or_else(|| anyhow!("tmux: empty output"))?;
    let mut it = line.split_whitespace();
    let a = it
        .next()
        .ok_or_else(|| anyhow!("tmux: bad output: {line}"))?
        .to_string();
    let b = it
        .next()
        .ok_or_else(|| anyhow!("tmux: bad output: {line}"))?
        .to_string();
    Ok((a, b))
}

/// Split a window. `target` should be a window_id like `@7` (or window target).
/// horizontal = panes side-by-side (-h), vertical = stacked (-v).
/// Returns the new pane's pane_id.
pub fn split_window(
    target: &str,
    cwd: &Path,
    horizontal: bool,
    percentage: Option<u8>,
    size: Option<u32>,
) -> Result<String> {
    let cwd_s = cwd.to_string_lossy().to_string();
    let mut args = vec![
        "split-window",
        "-t",
        target,
        "-c",
        cwd_s.as_str(),
        "-P",
        "-F",
        "#{pane_id}",
    ];
    if horizontal {
        args.push("-h");
    } else {
        args.push("-v");
    }
    let pct_s;
    let size_s;
    if let Some(p) = percentage {
        pct_s = format!("{}", p);
        args.push("-p");
        args.push(&pct_s);
    } else if let Some(sz) = size {
        size_s = format!("{}", sz);
        args.push("-l");
        args.push(&size_s);
    }
    let out = run(&args)?;
    Ok(out.lines().next().unwrap_or("").trim().to_string())
}

pub fn send_keys(target: &str, keys: &str) -> Result<()> {
    run(&["send-keys", "-t", target, keys, "Enter"])?;
    Ok(())
}

pub fn select_pane(target: &str) -> Result<()> {
    run(&["select-pane", "-t", target])?;
    Ok(())
}

pub fn select_window(target: &str) -> Result<()> {
    run(&["select-window", "-t", target])?;
    Ok(())
}

pub fn set_option(session: &str, key: &str, value: &str) -> Result<()> {
    run(&["set-option", "-t", session, key, value])?;
    Ok(())
}

/// Set a window option on every window of `session`.
/// (tmux per-session-global window options don't exist; we apply to each window.)
pub fn set_window_option_for_all(session: &str, key: &str, value: &str) -> Result<()> {
    let out = run(&["list-windows", "-t", session, "-F", "#{window_id}"])?;
    for wid in out.lines() {
        if wid.is_empty() {
            continue;
        }
        run(&["set-window-option", "-t", wid, key, value])?;
    }
    Ok(())
}

pub fn kill_session(name: &str) -> Result<()> {
    run(&["kill-session", "-t", name])?;
    Ok(())
}

pub fn list_sessions() -> Result<Vec<String>> {
    if !run_quiet(&["info"]) {
        // Server not running.
        return Ok(vec![]);
    }
    let out = run(&["list-sessions", "-F", "#{session_name}"])?;
    Ok(out.lines().map(|s| s.to_string()).collect())
}

/// Set a tmux user option (auto-prefixed with `@`) on a session.
pub fn set_user_option(session: &str, key: &str, value: &str) -> Result<()> {
    let opt = format!("@{key}");
    run(&["set-option", "-t", session, &opt, value])?;
    Ok(())
}

/// Set a pane-scoped user option (auto-prefixed with `@`). Requires tmux >= 3.0.
pub fn set_pane_option(pane: &str, key: &str, value: &str) -> Result<()> {
    let opt = format!("@{key}");
    run(&["set-option", "-p", "-t", pane, &opt, value])?;
    Ok(())
}

/// Read a pane-scoped user option (auto-prefixed with `@`); empty when unset.
pub fn get_pane_option(pane: &str, key: &str) -> String {
    let opt = format!("@{key}");
    run(&["show-options", "-pqv", "-t", pane, &opt])
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// `(session_name, window_name)` of the window that owns `pane`.
pub fn pane_location(pane: &str) -> Result<(String, String)> {
    let out = run(&[
        "display-message",
        "-p",
        "-t",
        pane,
        "#{session_name}\t#{window_name}",
    ])?;
    let line = out.lines().next().unwrap_or("");
    let mut it = line.split('\t');
    let session = it.next().unwrap_or("").to_string();
    let window = it.next().unwrap_or("").to_string();
    Ok((session, window))
}

/// ttys of attached clients — clients of `session`, or every client when
/// `None`. Best-effort: returns empty on any tmux error.
pub fn client_ttys(session: Option<&str>) -> Vec<String> {
    let target;
    let mut args = vec!["list-clients", "-F", "#{client_tty}"];
    if let Some(s) = session {
        target = format!("={s}");
        args.push("-t");
        args.push(&target);
    }
    match run(&args) {
        Ok(out) => out
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
        Err(_) => vec![],
    }
}

pub fn kill_pane(pane: &str) -> Result<()> {
    run(&["kill-pane", "-t", pane])?;
    Ok(())
}

/// Capture the last `lines` rows of `pane` as plain text.
pub fn capture_pane_tail(pane: &str, lines: u32) -> Result<String> {
    let start = format!("-{lines}");
    let out = run(&["capture-pane", "-p", "-t", pane, "-S", &start])?;
    Ok(out)
}

/// Capture the last `lines` rows of `pane` preserving ANSI escape sequences
/// (`-e`), so the dashboard preview keeps the agent's own colors.
pub fn capture_pane_ansi(pane: &str, lines: u32) -> Result<String> {
    let start = format!("-{lines}");
    let out = run(&["capture-pane", "-p", "-e", "-t", pane, "-S", &start])?;
    Ok(out)
}

/// One pane anywhere on the tmux server, with the agent-tracking options.
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: String,
    pub session: String,
    pub window_id: String,
    pub window_name: String,
    pub state_raw: String,
    pub seen: bool,
    pub current_cmd: String,
}

/// List every pane on the server (`list-panes -a`), with sessionx agent
/// options resolved. Used by the agent-mode dashboard.
pub fn list_all_panes() -> Result<Vec<PaneInfo>> {
    if !run_quiet(&["info"]) {
        // Server not running.
        return Ok(vec![]);
    }
    let fmt = "#{pane_id}\t#{session_name}\t#{window_id}\t#{window_name}\t#{@sx-agent-state}\t#{@sx-agent-seen}\t#{pane_current_command}";
    let out = run(&["list-panes", "-a", "-F", fmt])?;
    let mut v = vec![];
    for line in out.lines() {
        let mut it = line.split('\t');
        let id = it.next().unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        let session = it.next().unwrap_or("").to_string();
        let window_id = it.next().unwrap_or("").to_string();
        let window_name = it.next().unwrap_or("").to_string();
        let state_raw = it.next().unwrap_or("").to_string();
        let seen = it.next().unwrap_or("") == "1";
        let current_cmd = it.next().unwrap_or("").to_string();
        v.push(PaneInfo {
            id,
            session,
            window_id,
            window_name,
            state_raw,
            seen,
            current_cmd,
        });
    }
    Ok(v)
}

/// Clear a pane-scoped user option (auto-prefixed with `@`). Best-effort.
pub fn unset_pane_option(pane: &str, key: &str) {
    let opt = format!("@{key}");
    let _ = run_quiet(&["set-option", "-p", "-u", "-t", pane, &opt]);
}

/// Focus a pane from inside tmux: switch the client to its session, then
/// select its window and pane.
pub fn focus_pane(session: &str, window_id: &str, pane: &str) -> Result<()> {
    run(&["switch-client", "-t", &format!("={session}")])?;
    run(&["select-window", "-t", window_id])?;
    run(&["select-pane", "-t", pane])?;
    Ok(())
}

/// Attach to `session` focused on a specific window/pane, from outside tmux.
/// Blocks until the user detaches.
pub fn attach_at(session: &str, window_id: &str, pane: &str) -> Result<()> {
    let _ = run_quiet(&["select-window", "-t", window_id]);
    let _ = run_quiet(&["select-pane", "-t", pane]);
    let status = Command::new("tmux")
        .args(["attach-session", "-t", &format!("={session}")])
        .status()?;
    if !status.success() {
        return Err(anyhow!("tmux attach-session failed"));
    }
    Ok(())
}

#[derive(Debug)]
pub struct ManagedSession {
    pub name: String,
    pub project: String,
    pub handle: String,
    /// Plain tmux session (no `.sessionx.yaml`); has no worktree to clean up.
    pub plain: bool,
}

pub fn list_unmanaged_sessions() -> Result<Vec<String>> {
    if !run_quiet(&["info"]) {
        return Ok(vec![]);
    }
    let fmt = "#{session_name}\t#{@sessionx-managed}";
    let out = run(&["list-sessions", "-F", fmt])?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let mut it = line.split('\t');
            let name = it.next()?.to_string();
            let managed = it.next().unwrap_or("");
            if !name.is_empty() && managed != "1" {
                Some(name)
            } else {
                None
            }
        })
        .collect())
}

pub fn list_managed_sessions() -> Result<Vec<ManagedSession>> {
    if !run_quiet(&["info"]) {
        return Ok(vec![]);
    }
    let fmt = "#{session_name}\t#{@sessionx-managed}\t#{@sessionx-project}\t#{@sessionx-handle}\t#{@sessionx-plain}";
    let out = run(&["list-sessions", "-F", fmt])?;
    let mut v = vec![];
    for line in out.lines() {
        let mut it = line.split('\t');
        let name = it.next().unwrap_or("").to_string();
        let managed = it.next().unwrap_or("");
        let project = it.next().unwrap_or("").to_string();
        let handle = it.next().unwrap_or("").to_string();
        let plain = it.next().unwrap_or("") == "1";
        if managed == "1" && !name.is_empty() {
            v.push(ManagedSession {
                name,
                project,
                handle,
                plain,
            });
        }
    }
    Ok(v)
}

pub fn in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// Returns true if the current tmux session was tagged by sessionx as managed.
pub fn current_session_is_managed() -> bool {
    if !in_tmux() {
        return false;
    }
    let out = match Command::new("tmux")
        .args(["display-message", "-p", "#{@sessionx-managed}"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };
    String::from_utf8_lossy(&out.stdout).trim() == "1"
}

/// Returns true if the current process is running inside a sessionx-attached tmux session.
/// Combines two signals:
///  - the current session is managed (`@sessionx-managed=1`), or
///  - `SESSIONX_ACTIVE` is set in the environment (covers unmanaged sessions sessionx attached to).
pub fn in_sessionx() -> bool {
    current_session_is_managed() || std::env::var("SESSIONX_ACTIVE").is_ok()
}

pub fn attach_or_switch(name: &str) -> Result<()> {
    // Mark the tmux server env so child shells in attached sessions can detect sessionx.
    let _ = run_quiet(&["set-environment", "-g", "SESSIONX_ACTIVE", "1"]);
    if in_tmux() {
        run(&["switch-client", "-t", name])?;
    } else {
        // attach-session replaces the current process so we exec.
        let err = std::process::Command::new("tmux")
            .args(["attach-session", "-t", name])
            .status()?;
        if !err.success() {
            return Err(anyhow!("tmux attach-session failed"));
        }
    }
    Ok(())
}
