use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const DEFAULT_REPO: &str = "https://github.com/jeromecoloma/sessionx-hooks";
pub const PINNED_REF: &str = "v0.1.0";

pub fn repo_url() -> String {
    std::env::var("SX_HOOKS_REPO").unwrap_or_else(|_| DEFAULT_REPO.into())
}

pub fn pinned_ref() -> String {
    std::env::var("SX_HOOKS_REF").unwrap_or_else(|_| PINNED_REF.into())
}

/// Cache holding the cloned sessionx-hooks repo (`~/.sessionx/hooks-cache`).
pub fn cache_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home dir"))?;
    Ok(home.join(".sessionx/hooks-cache"))
}

/// Where installed recipe scripts live (`~/.sessionx/scripts/<id>/`).
pub fn install_dir(recipe_id: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home dir"))?;
    Ok(home.join(".sessionx/scripts").join(recipe_id))
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    #[allow(dead_code)]
    pub schema_version: u32,
    #[allow(dead_code)]
    pub sessionx_min_version: String,
    pub recipes: Vec<RecipeMeta>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RecipeMeta {
    pub id: String,
    pub description: String,
    pub stack: String,
    pub post_create: String,
    pub pre_remove: String,
    #[serde(default)]
    pub required_env: Vec<String>,
    #[serde(default)]
    pub optional_env: Vec<String>,
    #[serde(default)]
    pub secrets: Vec<String>,
    #[serde(default)]
    pub requires_bins: Vec<String>,
    /// When true, the recipe only makes sense in worktree mode and the init
    /// wizard hides it when the user opts out of worktrees.
    #[serde(default)]
    pub requires_worktree: bool,
}

/// Ensure the hooks repo is cloned into the cache dir at the pinned ref.
/// Idempotent: if already cloned, fetches and checks out the pinned ref.
pub fn ensure_cloned() -> Result<PathBuf> {
    let dir = cache_dir()?;
    let url = repo_url();
    let r#ref = pinned_ref();

    if dir.join(".git").is_dir() {
        // Existing clone — make sure it's pointing at the requested ref.
        let cur = current_ref(&dir).unwrap_or_default();
        if cur != r#ref {
            run_git(&dir, &["fetch", "--tags", "--quiet", "origin"])
                .context("git fetch")?;
            run_git(&dir, &["checkout", "--quiet", &r#ref]).context("git checkout")?;
        }
        return Ok(dir);
    }

    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Clone. `--branch` accepts tags too. Suppress the detached-HEAD advice.
    let status = Command::new("git")
        .args([
            "-c",
            "advice.detachedHead=false",
            "clone",
            "--depth",
            "1",
            "--branch",
            &r#ref,
            "--quiet",
            &url,
            &dir.display().to_string(),
        ])
        .status()
        .context("running git clone")?;
    if !status.success() {
        return Err(anyhow!(
            "git clone {url} (branch={ref}) failed; set SX_HOOKS_REPO/SX_HOOKS_REF to override",
            url = url,
            ref = r#ref
        ));
    }
    Ok(dir)
}

fn current_ref(dir: &Path) -> Result<String> {
    // Try tag first, fall back to commit.
    let out = Command::new("git")
        .args(["-C", &dir.display().to_string(), "describe", "--tags", "--exact-match"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    let out = Command::new("git")
        .args(["-C", &dir.display().to_string(), "rev-parse", "HEAD"])
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "advice.detachedHead=false"])
        .arg("-C")
        .arg(dir)
        .args(args);
    let status = cmd.status()?;
    if !status.success() {
        return Err(anyhow!("git {args:?} failed in {}", dir.display()));
    }
    Ok(())
}

pub fn read_manifest(cache: &Path) -> Result<Manifest> {
    let path = cache.join("recipes.yaml");
    let body = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let m: Manifest = serde_yaml::from_str(&body)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(m)
}

pub fn find_recipe<'a>(m: &'a Manifest, id: &str) -> Result<&'a RecipeMeta> {
    m.recipes
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| anyhow!("no such recipe '{id}' (try `sessionx hooks list`)"))
}

pub struct Installed {
    pub recipe: RecipeMeta,
    pub post_create: PathBuf,
    pub pre_remove: PathBuf,
}

impl Installed {
    /// Path expressed using `~/...` for portability into `.sessionx.yaml`.
    pub fn post_create_tilde(&self) -> String {
        tilde_path(&self.post_create)
    }
    pub fn pre_remove_tilde(&self) -> String {
        tilde_path(&self.pre_remove)
    }
}

fn tilde_path(p: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = p.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
    }
    p.display().to_string()
}

/// Copy the recipe's directory + the shared `lib/` into `~/.sessionx/scripts/<id>/`.
/// Returns paths to the installed setup/teardown scripts.
pub fn install_recipe(id: &str) -> Result<Installed> {
    let cache = ensure_cloned()?;
    let m = read_manifest(&cache)?;
    let recipe = find_recipe(&m, id)?.clone();

    let target = install_dir(id)?;
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .with_context(|| format!("clearing {}", target.display()))?;
    }
    std::fs::create_dir_all(&target)?;

    // Copy the recipe directory contents.
    let src_recipe = cache.join("recipes").join(id);
    if !src_recipe.is_dir() {
        return Err(anyhow!(
            "recipe dir missing in cache: {}",
            src_recipe.display()
        ));
    }
    copy_dir_all(&src_recipe, &target)?;

    // Copy the shared lib alongside, so `source ../../lib/env.sh` from the
    // recipe still resolves once installed.
    let src_lib = cache.join("lib");
    if src_lib.is_dir() {
        // Mirror the `recipes/<id>/` → install_dir mapping by placing lib/ two
        // levels up from the script (target is `~/.sessionx/scripts/<id>`, so
        // `../../lib` resolves to `~/.sessionx/lib`).
        let lib_dst = target
            .parent()
            .ok_or_else(|| anyhow!("install path has no parent"))?
            .parent()
            .ok_or_else(|| anyhow!("install path has no grandparent"))?
            .join("lib");
        std::fs::create_dir_all(&lib_dst)?;
        copy_dir_all(&src_lib, &lib_dst)?;
    }

    // Make .sh files executable.
    chmod_executables(&target)?;

    let post_create = resolve_script_path(&target, &cache, id, &recipe.post_create)?;
    let pre_remove = resolve_script_path(&target, &cache, id, &recipe.pre_remove)?;

    Ok(Installed {
        recipe,
        post_create,
        pre_remove,
    })
}

/// Manifest entries are repo-relative (e.g. `recipes/laravel-herd/setup.sh`).
/// Map them to the installed location under `target` (which holds the recipe's contents directly).
fn resolve_script_path(
    target: &Path,
    _cache: &Path,
    id: &str,
    manifest_rel: &str,
) -> Result<PathBuf> {
    let prefix = format!("recipes/{id}/");
    let stripped = manifest_rel
        .strip_prefix(&prefix)
        .ok_or_else(|| anyhow!("recipe script '{manifest_rel}' is outside recipes/{id}/"))?;
    Ok(target.join(stripped))
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            std::fs::copy(&from, &to)
                .with_context(|| format!("copy {} → {}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn chmod_executables(dir: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for entry in walkdir(dir)? {
        if entry.extension().and_then(|s| s.to_str()) == Some("sh") {
            let mut perms = std::fs::metadata(&entry)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&entry, perms)?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn chmod_executables(_dir: &Path) -> Result<()> {
    Ok(())
}

fn walkdir(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        for entry in std::fs::read_dir(&p)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    Ok(out)
}

/// Check which `requires_bins` are missing from PATH. Returns the missing ones.
pub fn missing_bins(recipe: &RecipeMeta) -> Vec<String> {
    recipe
        .requires_bins
        .iter()
        .filter(|bin| {
            let s = Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {bin}"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            !matches!(s, Ok(st) if st.success())
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses() {
        let yaml = r#"
schema_version: 1
sessionx_min_version: "0.1.0"
recipes:
  - id: laravel-herd
    description: Test
    stack: php
    post_create: recipes/laravel-herd/setup.sh
    pre_remove:  recipes/laravel-herd/teardown.sh
    required_env: [SX_LARAVEL_DIR]
    optional_env: [SX_RUN_MIGRATIONS]
    secrets: [SX_DB_ADMIN_USER]
    requires_bins: [git, php]
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.schema_version, 1);
        assert_eq!(m.recipes.len(), 1);
        let r = &m.recipes[0];
        assert_eq!(r.id, "laravel-herd");
        assert_eq!(r.stack, "php");
        assert_eq!(r.required_env, vec!["SX_LARAVEL_DIR"]);
        assert_eq!(r.requires_bins, vec!["git", "php"]);
    }

    #[test]
    fn manifest_minimal() {
        let yaml = r#"
schema_version: 1
sessionx_min_version: "0.1.0"
recipes:
  - id: foo
    description: bar
    stack: generic
    post_create: recipes/foo/setup.sh
    pre_remove: recipes/foo/teardown.sh
"#;
        let m: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert!(m.recipes[0].required_env.is_empty());
        assert!(m.recipes[0].requires_bins.is_empty());
    }

    #[test]
    fn env_overrides_apply() {
        std::env::set_var("SX_HOOKS_REPO", "https://example.test/foo.git");
        std::env::set_var("SX_HOOKS_REF", "v9.9.9");
        assert_eq!(repo_url(), "https://example.test/foo.git");
        assert_eq!(pinned_ref(), "v9.9.9");
        std::env::remove_var("SX_HOOKS_REPO");
        std::env::remove_var("SX_HOOKS_REF");
    }

    #[test]
    fn resolve_script_path_strips_repo_prefix() {
        let target = std::path::Path::new("/x/scripts/laravel-herd");
        let cache = std::path::Path::new("/cache");
        let p = resolve_script_path(target, cache, "laravel-herd", "recipes/laravel-herd/setup.sh")
            .unwrap();
        assert_eq!(p, std::path::Path::new("/x/scripts/laravel-herd/setup.sh"));
    }

    #[test]
    fn resolve_script_path_rejects_outside() {
        let target = std::path::Path::new("/x");
        let cache = std::path::Path::new("/cache");
        assert!(resolve_script_path(target, cache, "laravel-herd", "lib/env.sh").is_err());
    }
}
