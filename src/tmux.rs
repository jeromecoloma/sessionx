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

/// Returns `(window_id, pane_id)` for the session's first window/pane.
pub fn new_session(name: &str, cwd: &Path, first_window_name: Option<&str>) -> Result<(String, String)> {
    let cwd_s = cwd.to_string_lossy().to_string();
    let mut args = vec![
        "new-session", "-d", "-s", name, "-c", cwd_s.as_str(),
        "-P", "-F", "#{window_id} #{pane_id}",
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
        "new-window", "-t", session, "-c", cwd_s.as_str(),
        "-P", "-F", "#{window_id} #{pane_id}",
    ];
    if let Some(n) = name {
        args.push("-n");
        args.push(n);
    }
    let out = run(&args)?;
    parse_two(&out)
}

fn parse_two(out: &str) -> Result<(String, String)> {
    let line = out.lines().next().ok_or_else(|| anyhow!("tmux: empty output"))?;
    let mut it = line.split_whitespace();
    let a = it.next().ok_or_else(|| anyhow!("tmux: bad output: {line}"))?.to_string();
    let b = it.next().ok_or_else(|| anyhow!("tmux: bad output: {line}"))?.to_string();
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
        "split-window", "-t", target, "-c", cwd_s.as_str(),
        "-P", "-F", "#{pane_id}",
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

pub fn in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

pub fn attach_or_switch(name: &str) -> Result<()> {
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
