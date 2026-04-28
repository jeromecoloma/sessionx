use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

fn verbose() -> bool {
    std::env::var("SX_VERBOSE").ok().as_deref() == Some("1")
}

pub struct HookEnv {
    pub vars: Vec<(String, String)>,
    pub cwd: std::path::PathBuf,
}

/// Run a list of bash strings in order, aborting on first failure.
pub fn run_all(label: &str, commands: &[String], env: &HookEnv) -> Result<()> {
    for cmd in commands {
        if verbose() {
            eprintln!("+ [{label}] bash -lc {cmd:?}  (cwd={})", env.cwd.display());
        }
        let mut c = Command::new("bash");
        c.arg("-lc").arg(cmd).current_dir(&env.cwd);
        for (k, v) in &env.vars {
            c.env(k, v);
        }
        let status = c.status()?;
        if !status.success() {
            return Err(anyhow!("hook ({label}) failed: {cmd}"));
        }
    }
    Ok(())
}

pub fn base_env(
    project_root: &Path,
    handle: &str,
    session_name: &str,
    worktree: Option<&Path>,
    branch: Option<&str>,
) -> Vec<(String, String)> {
    let mut v = vec![
        ("SX_PROJECT_ROOT".into(), project_root.display().to_string()),
        ("SX_HANDLE".into(), handle.into()),
        ("SX_SESSION_NAME".into(), session_name.into()),
    ];
    if let Some(p) = worktree {
        v.push(("SX_WORKTREE_PATH".into(), p.display().to_string()));
    }
    if let Some(b) = branch {
        v.push(("SX_BRANCH_NAME".into(), b.into()));
    }
    v
}
