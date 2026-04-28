use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const CONFIG_FILENAME: &str = ".sessionx.yaml";

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // Reserved: window-mode not yet implemented; field kept so configs validate.
    #[serde(default)]
    #[allow(dead_code)]
    pub mode: Mode,

    pub worktree_dir: Option<String>,

    #[serde(default)]
    pub worktree_naming: WorktreeNaming,

    pub session_prefix: Option<String>,

    pub windows: Option<Vec<WindowSpec>>,
    pub panes: Option<Vec<PaneSpec>>,

    #[serde(default)]
    pub post_create: Vec<String>,
    #[serde(default)]
    pub pre_remove: Vec<String>,

    #[serde(default)]
    pub files: FilesSpec,

    #[serde(default)]
    pub status: StatusSpec,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Session,
    Window,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorktreeNaming {
    #[default]
    Full,
    Basename,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowSpec {
    pub name: Option<String>,
    pub panes: Vec<PaneSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneSpec {
    pub command: Option<String>,
    #[serde(default)]
    pub focus: bool,
    pub split: Option<SplitDir>,
    pub percentage: Option<u8>,
    pub size: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SplitDir {
    Horizontal,
    Vertical,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct FilesSpec {
    #[serde(default)]
    pub copy: Vec<String>,
    #[serde(default)]
    pub symlink: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct StatusSpec {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub style: std::collections::BTreeMap<String, String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub window_format: Option<String>,
    pub current_window_format: Option<String>,
    #[serde(default)]
    pub segments: Vec<SegmentSpec>,
    pub status_interval: Option<u32>,
    #[serde(default)]
    pub icons: std::collections::BTreeMap<String, String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentSpec {
    pub name: String,
    pub command: String,
    // Per-segment interval is parsed for forward-compat; today tmux's #(...)
    // cache is governed by the session-wide `status_interval`.
    #[serde(default)]
    #[allow(dead_code)]
    pub interval: Option<u32>,
}

pub struct Loaded {
    pub config: Config,
    pub project_root: PathBuf,
    pub config_path: PathBuf,
}

impl Loaded {
    pub fn worktree_mode(&self) -> bool {
        self.config.worktree_dir.is_some()
    }

    pub fn session_prefix(&self) -> String {
        if let Some(p) = &self.config.session_prefix {
            return sanitize_session(p);
        }
        let base = self
            .project_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("sx");
        sanitize_session(&format!("{base}-"))
    }

    pub fn session_name(&self, handle: &str) -> String {
        sanitize_session(&format!("{}{}", self.session_prefix(), handle))
    }
}

/// tmux target syntax uses `.` and `:` as separators, so they're unsafe in session names.
fn sanitize_session(s: &str) -> String {
    s.chars()
        .map(|c| if matches!(c, '.' | ':' | ' ' | '\t') { '_' } else { c })
        .collect()
}

/// Walk up from `cwd` looking for `.sessionx.yaml`.
pub fn find_and_load() -> Result<Loaded> {
    let cwd = std::env::current_dir()?;
    let mut dir: &Path = &cwd;
    loop {
        let candidate = dir.join(CONFIG_FILENAME);
        if candidate.is_file() {
            let content = std::fs::read_to_string(&candidate)
                .with_context(|| format!("reading {}", candidate.display()))?;
            let config: Config = serde_yaml::from_str(&content)
                .with_context(|| format!("parsing {}", candidate.display()))?;
            validate(&config)?;
            return Ok(Loaded {
                config,
                project_root: dir.to_path_buf(),
                config_path: candidate,
            });
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => return Err(anyhow!(
                "no {} found in {} or any parent directory",
                CONFIG_FILENAME,
                cwd.display()
            )),
        }
    }
}

fn validate(c: &Config) -> Result<()> {
    if c.windows.is_some() && c.panes.is_some() {
        return Err(anyhow!("config: 'windows' and 'panes' are mutually exclusive"));
    }
    Ok(())
}
