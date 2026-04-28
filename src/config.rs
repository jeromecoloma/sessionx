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

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct StatusSpec {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional preset theme name. User-provided keys below override theme defaults.
    pub theme: Option<String>,
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

#[derive(Debug, Deserialize, Clone)]
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

/// Rewrite the `theme:` value under `status:` in a .sessionx.yaml file in place.
/// Preserves comments, blank lines, and other keys. Atomic write via tempfile + rename.
pub fn set_theme_in_file(path: &Path, theme: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let updated = rewrite_theme(&content, theme);

    let tmp = path.with_extension("yaml.tmp");
    std::fs::write(&tmp, &updated)
        .with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} to {}", tmp.display(), path.display()))?;
    Ok(())
}

fn rewrite_theme(input: &str, theme: &str) -> String {
    let lines: Vec<&str> = input.split_inclusive('\n').collect();

    // Find top-level `status:` line.
    let status_idx = lines.iter().position(|l| {
        let no_nl = l.trim_end_matches('\n');
        no_nl == "status:" || no_nl.starts_with("status:") && !no_nl.starts_with(' ') && !no_nl.starts_with('\t')
    });

    let Some(status_idx) = status_idx else {
        // No status block — append one.
        let mut out = input.to_string();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(&format!("status:\n  theme: {theme}\n"));
        return out;
    };

    // Determine end of status block: next line that is non-empty and starts at col 0 (top-level key).
    let mut end_idx = lines.len();
    for (i, l) in lines.iter().enumerate().skip(status_idx + 1) {
        let trimmed = l.trim_end_matches('\n');
        if trimmed.is_empty() {
            continue;
        }
        // top-level key (no leading whitespace) and not a comment continuation
        if !trimmed.starts_with(' ') && !trimmed.starts_with('\t') {
            end_idx = i;
            break;
        }
    }

    // Search within [status_idx+1, end_idx) for a theme line (uncommented or commented).
    let theme_re_uncommented = |s: &str| -> bool {
        let t = s.trim_start();
        t.starts_with("theme:") && s.starts_with(' ')
    };
    let theme_re_commented = |s: &str| -> bool {
        let t = s.trim_start();
        if !t.starts_with('#') {
            return false;
        }
        let after = t.trim_start_matches('#').trim_start();
        after.starts_with("theme:")
    };

    let mut out = String::with_capacity(input.len() + 32);
    let mut wrote = false;
    let new_line = format!("  theme: {theme}\n");

    for (i, l) in lines.iter().enumerate() {
        if i > status_idx && i < end_idx && !wrote {
            let no_nl = l.trim_end_matches('\n');
            if theme_re_uncommented(no_nl) || theme_re_commented(no_nl) {
                out.push_str(&new_line);
                wrote = true;
                continue;
            }
        }
        out.push_str(l);
    }

    if !wrote {
        // Insert as the first child line right after `status:`.
        let mut out2 = String::with_capacity(out.len() + new_line.len());
        for (i, l) in out.split_inclusive('\n').enumerate() {
            out2.push_str(l);
            if i == status_idx {
                out2.push_str(&new_line);
            }
        }
        return out2;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::rewrite_theme;

    #[test]
    fn replaces_existing_theme() {
        let input = "status:\n  theme: tokyo-night\n  enabled: true\n";
        let out = rewrite_theme(input, "dracula");
        assert_eq!(out, "status:\n  theme: dracula\n  enabled: true\n");
    }

    #[test]
    fn uncomments_commented_theme() {
        let input = "status:\n  enabled: true\n  # theme: tokyo-night\n  left: \" #S \"\n";
        let out = rewrite_theme(input, "nord");
        assert_eq!(out, "status:\n  enabled: true\n  theme: nord\n  left: \" #S \"\n");
    }

    #[test]
    fn inserts_when_missing_under_status() {
        let input = "status:\n  enabled: true\n  left: \" #S \"\n";
        let out = rewrite_theme(input, "gruvbox");
        assert_eq!(out, "status:\n  theme: gruvbox\n  enabled: true\n  left: \" #S \"\n");
    }

    #[test]
    fn appends_when_no_status_block() {
        let input = "mode: session\nworktree_dir: .worktrees\n";
        let out = rewrite_theme(input, "rose-pine");
        assert!(out.ends_with("status:\n  theme: rose-pine\n"));
        assert!(out.contains("mode: session"));
    }

    #[test]
    fn preserves_comments() {
        let input = "# top comment\nstatus:\n  # nested comment\n  theme: tokyo-night\n  # trailing comment\n";
        let out = rewrite_theme(input, "catppuccin");
        assert!(out.contains("# top comment"));
        assert!(out.contains("# nested comment"));
        assert!(out.contains("# trailing comment"));
        assert!(out.contains("theme: catppuccin"));
        assert!(!out.contains("tokyo-night"));
    }
}
