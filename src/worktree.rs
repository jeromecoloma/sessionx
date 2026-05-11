use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{FilesSpec, Loaded, WorktreeNaming};

fn verbose() -> bool {
    std::env::var("SX_VERBOSE").ok().as_deref() == Some("1")
}

fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    if verbose() {
        eprintln!("+ git -C {} {}", cwd.display(), args.join(" "));
    }
    let out = Command::new("git").current_dir(cwd).args(args).output()?;
    if !out.status.success() {
        return Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn handle_to_branch(handle: &str, _naming: WorktreeNaming) -> String {
    // For now branch == handle. `naming` only affects how we *display* it later.
    handle.to_string()
}

pub fn worktree_path(loaded: &Loaded, handle: &str) -> Result<PathBuf> {
    let dir = loaded
        .config
        .worktree_dir
        .as_ref()
        .ok_or_else(|| anyhow!("worktree_dir not set"))?;
    let leaf = match loaded.config.worktree_naming {
        WorktreeNaming::Full => handle.replace('/', "-"),
        WorktreeNaming::Basename => handle.rsplit('/').next().unwrap_or(handle).to_string(),
    };
    let base = Path::new(dir);
    let abs = if base.is_absolute() {
        base.to_path_buf()
    } else {
        loaded.project_root.join(base)
    };
    Ok(abs.join(leaf))
}

pub fn create(loaded: &Loaded, handle: &str, base: Option<&str>) -> Result<PathBuf> {
    let path = worktree_path(loaded, handle)?;
    if path.exists() {
        return Err(anyhow!("worktree path already exists: {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Prune stale registrations — handles the case where a worktree dir was
    // removed manually (e.g. `rm -rf`) without `git worktree remove`, which
    // would otherwise make `add` fail with "missing but already registered".
    let _ = git(&loaded.project_root, &["worktree", "prune"]);

    let branch = handle_to_branch(handle, loaded.config.worktree_naming);
    let path_s = path.to_string_lossy().to_string();

    // If the branch already exists, check it out into the worktree as-is.
    // Otherwise create it (optionally based on `base`).
    let args: Vec<&str> = if branch_exists(&loaded.project_root, &branch) {
        if base.is_some() {
            return Err(anyhow!(
                "branch '{branch}' already exists; refusing to use --base (would be ignored). \
                 Pick a new handle, or omit --base."
            ));
        }
        vec!["worktree", "add", &path_s, &branch]
    } else {
        let mut a = vec!["worktree", "add", "-b", &branch, &path_s];
        if let Some(b) = base {
            a.push(b);
        }
        a
    };

    git(&loaded.project_root, &args)?;
    apply_files(loaded, &path)?;
    Ok(path)
}

fn branch_exists(repo: &Path, branch: &str) -> bool {
    Command::new("git")
        .current_dir(repo)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn remove(loaded: &Loaded, handle: &str, force: bool) -> Result<()> {
    let path = worktree_path(loaded, handle)?;
    if !path.exists() {
        return Ok(());
    }
    let path_s = path.to_string_lossy().to_string();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path_s);
    match git(&loaded.project_root, &args) {
        Ok(_) => {}
        Err(e) if force && is_not_a_worktree(&e) => {
            // Stale on-disk dir not registered with git (e.g. created via `mkdir`
            // or left behind after a prior partial removal). Force-mode means
            // the user has explicitly opted into destruction — wipe the dir and
            // prune any dangling registration.
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("rm -rf {} after git refused", path.display()))?;
            let _ = git(&loaded.project_root, &["worktree", "prune"]);
        }
        Err(e) if is_dir_not_empty(&e) => {
            // Git's safety checks passed (no modified/untracked tracked files)
            // but rmdir failed on gitignored leftovers — vendor/, node_modules/,
            // logs written by pre_remove teardown hooks, etc. Safe to wipe.
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("rm -rf {} after git refused", path.display()))?;
            let _ = git(&loaded.project_root, &["worktree", "prune"]);
        }
        Err(e) => return Err(e),
    }
    let branch = handle_to_branch(handle, loaded.config.worktree_naming);
    // Best-effort branch delete; ignore failure (e.g. unmerged without force).
    let _ = git(
        &loaded.project_root,
        &["branch", if force { "-D" } else { "-d" }, &branch],
    );
    Ok(())
}

fn is_not_a_worktree(err: &anyhow::Error) -> bool {
    err.to_string().contains("is not a working tree")
}

fn is_dir_not_empty(err: &anyhow::Error) -> bool {
    err.to_string().contains("Directory not empty")
}

fn apply_files(loaded: &Loaded, dest: &Path) -> Result<()> {
    let spec: &FilesSpec = &loaded.config.files;
    for rel in &spec.copy {
        let src = loaded.project_root.join(rel);
        if !src.exists() {
            continue;
        }
        let dst = dest.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} → {}", src.display(), dst.display()))?;
    }
    for rel in &spec.symlink {
        let src = loaded.project_root.join(rel);
        if !src.exists() {
            continue;
        }
        let dst = dest.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if dst.exists() || dst.symlink_metadata().is_ok() {
            continue;
        }
        std::os::unix::fs::symlink(&src, &dst)
            .with_context(|| format!("symlink {} → {}", src.display(), dst.display()))?;
    }
    Ok(())
}

pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .current_dir(path)
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
