use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Rust,
    Node,
    Php,
    Python,
    Generic,
}

impl ProjectKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Node => "Node",
            Self::Php => "PHP",
            Self::Python => "Python",
            Self::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub kind: ProjectKind,
    /// Subdirectory the project markers live in (e.g. "www" for Laravel Herd).
    /// `None` means project root.
    pub subdir: Option<String>,
}

pub fn detect(cwd: &Path) -> Detected {
    if let Some(d) = detect_at(cwd, None) {
        return d;
    }
    // Laravel-Herd-style: code lives under ./www/.
    if let Some(d) = detect_at(cwd, Some("www")) {
        return d;
    }
    Detected {
        kind: ProjectKind::Generic,
        subdir: None,
    }
}

fn detect_at(cwd: &Path, subdir: Option<&str>) -> Option<Detected> {
    let base = match subdir {
        Some(s) => cwd.join(s),
        None => cwd.to_path_buf(),
    };
    let has = |name: &str| base.join(name).exists();
    // composer.json wins over package.json: Laravel apps ship a frontend
    // package.json (Vite/Mix) but are PHP projects.
    let kind = if has("Cargo.toml") {
        ProjectKind::Rust
    } else if has("composer.json") {
        ProjectKind::Php
    } else if has("package.json") {
        ProjectKind::Node
    } else if has("pyproject.toml") || has("requirements.txt") {
        ProjectKind::Python
    } else {
        return None;
    };
    Some(Detected {
        kind,
        subdir: subdir.map(|s| s.to_string()),
    })
}

/// Build a `windows:` YAML block (sans the `windows:` key itself) for the given kind.
/// Returns `None` for `Generic` so the template's default block is kept as-is.
pub fn windows_yaml_for(d: &Detected, cwd: &Path) -> Option<String> {
    match d.kind {
        ProjectKind::Generic => None,
        ProjectKind::Rust => Some(rust_windows(cwd)),
        ProjectKind::Node => Some(node_windows(cwd, d.subdir.as_deref())),
        ProjectKind::Php => Some(php_windows(cwd, d.subdir.as_deref())),
        ProjectKind::Python => Some(python_windows(d.subdir.as_deref())),
    }
}

fn rust_windows(_cwd: &Path) -> String {
    let run_cmd = if has_bin("cargo-watch") {
        "cargo watch -x run"
    } else {
        "cargo run"
    };
    format!(
        "  - name: shell\n\
         \x20   panes:\n\
         \x20     - command: exec $SHELL\n\
         \x20       focus: true\n\
         \x20 - name: edit\n\
         \x20   panes:\n\
         \x20     - command: ${{EDITOR:-vi}} .\n\
         \x20 - name: run\n\
         \x20   panes:\n\
         \x20     - command: {run_cmd}\n"
    )
}

fn edit_target(subdir: Option<&str>) -> String {
    subdir.map(|s| s.to_string()).unwrap_or_else(|| ".".into())
}

fn cd_prefix(subdir: Option<&str>) -> String {
    subdir.map(|s| format!("cd {s} && ")).unwrap_or_default()
}

fn node_windows(cwd: &Path, subdir: Option<&str>) -> String {
    let pkg_dir = match subdir {
        Some(s) => cwd.join(s),
        None => cwd.to_path_buf(),
    };
    let dev_cmd = node_dev_command(&pkg_dir);
    let edit = edit_target(subdir);
    let mut out = format!(
        "  - name: shell\n\
         \x20   panes:\n\
         \x20     - command: exec $SHELL\n\
         \x20       focus: true\n\
         \x20 - name: edit\n\
         \x20   panes:\n\
         \x20     - command: ${{EDITOR:-vi}} {edit}\n",
    );
    if let Some(cmd) = dev_cmd {
        let prefix = cd_prefix(subdir);
        out.push_str(&format!(
            "  - name: dev\n\
             \x20   panes:\n\
             \x20     - command: {prefix}{cmd}\n"
        ));
    }
    out
}

fn php_windows(_cwd: &Path, _subdir: Option<&str>) -> String {
    // First window runs your AI agent (claude/codex/aider/...). The `<agent>`
    // placeholder resolves via SX_AGENT, ~/.config/sessionx/config.yaml, or
    // defaults to "claude".
    // Edit window opens at the worktree/project root (not the Laravel subdir).
    // No `serve` window: Laravel Herd serves the app — `php artisan serve`
    // would conflict.
    "  - name: agent\n\
     \x20   panes:\n\
     \x20     - command: <agent>\n\
     \x20       focus: true\n\
     \x20 - name: edit\n\
     \x20   panes:\n\
     \x20     - command: ${EDITOR:-vi} .\n"
        .to_string()
}

fn python_windows(subdir: Option<&str>) -> String {
    let edit = edit_target(subdir);
    format!(
        "  - name: shell\n\
         \x20   panes:\n\
         \x20     - command: exec $SHELL\n\
         \x20       focus: true\n\
         \x20 - name: edit\n\
         \x20   panes:\n\
         \x20     - command: ${{EDITOR:-vi}} {edit}\n",
    )
}

fn node_dev_command(cwd: &Path) -> Option<String> {
    let pkg = std::fs::read_to_string(cwd.join("package.json")).ok()?;
    let scripts = extract_scripts_block(&pkg)?;
    if scripts.contains("\"dev\"") {
        Some("npm run dev".into())
    } else if scripts.contains("\"start\"") {
        Some("npm start".into())
    } else {
        None
    }
}

/// Crude extraction of the `"scripts": { ... }` block from package.json without pulling in serde_json.
fn extract_scripts_block(pkg: &str) -> Option<String> {
    let key_idx = pkg.find("\"scripts\"")?;
    let rest = &pkg[key_idx..];
    let open = rest.find('{')?;
    let mut depth = 0i32;
    let mut end = None;
    for (i, ch) in rest[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    Some(rest[open..end?].to_string())
}

fn has_bin(bin: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {bin}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let c = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let p = std::env::temp_dir().join(format!("sessionx-init-test-{pid}-{n}-{c}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn detects_rust() {
        let d = tmp();
        fs::write(d.join("Cargo.toml"), "[package]\n").unwrap();
        let r = detect(&d);
        assert_eq!(r.kind, ProjectKind::Rust);
        assert_eq!(r.subdir, None);
    }

    #[test]
    fn detects_node() {
        let d = tmp();
        fs::write(d.join("package.json"), "{}").unwrap();
        let r = detect(&d);
        assert_eq!(r.kind, ProjectKind::Node);
        assert_eq!(r.subdir, None);
    }

    #[test]
    fn detects_php() {
        let d = tmp();
        fs::write(d.join("composer.json"), "{}").unwrap();
        let r = detect(&d);
        assert_eq!(r.kind, ProjectKind::Php);
        assert_eq!(r.subdir, None);
    }

    #[test]
    fn php_wins_over_node_when_both_present() {
        let d = tmp();
        fs::write(d.join("composer.json"), "{}").unwrap();
        fs::write(d.join("package.json"), "{}").unwrap();
        let r = detect(&d);
        assert_eq!(r.kind, ProjectKind::Php);
    }

    #[test]
    fn php_wins_in_www_when_both_present() {
        let d = tmp();
        fs::create_dir_all(d.join("www")).unwrap();
        fs::write(d.join("www/composer.json"), "{}").unwrap();
        fs::write(d.join("www/package.json"), "{}").unwrap();
        fs::write(d.join("www/artisan"), "").unwrap();
        let r = detect(&d);
        assert_eq!(r.kind, ProjectKind::Php);
        assert_eq!(r.subdir.as_deref(), Some("www"));
    }

    #[test]
    fn detects_php_in_www() {
        let d = tmp();
        fs::create_dir_all(d.join("www")).unwrap();
        fs::write(d.join("www/composer.json"), "{}").unwrap();
        fs::write(d.join("www/artisan"), "").unwrap();
        let r = detect(&d);
        assert_eq!(r.kind, ProjectKind::Php);
        assert_eq!(r.subdir.as_deref(), Some("www"));
    }

    #[test]
    fn root_takes_precedence_over_www() {
        let d = tmp();
        fs::write(d.join("composer.json"), "{}").unwrap();
        fs::create_dir_all(d.join("www")).unwrap();
        fs::write(d.join("www/composer.json"), "{}").unwrap();
        let r = detect(&d);
        assert_eq!(r.kind, ProjectKind::Php);
        assert_eq!(r.subdir, None);
    }

    #[test]
    fn detects_python_pyproject() {
        let d = tmp();
        fs::write(d.join("pyproject.toml"), "").unwrap();
        assert_eq!(detect(&d).kind, ProjectKind::Python);
    }

    #[test]
    fn detects_python_requirements() {
        let d = tmp();
        fs::write(d.join("requirements.txt"), "").unwrap();
        assert_eq!(detect(&d).kind, ProjectKind::Python);
    }

    #[test]
    fn detects_generic() {
        let d = tmp();
        assert_eq!(detect(&d).kind, ProjectKind::Generic);
    }

    #[test]
    fn rust_takes_precedence_over_node() {
        let d = tmp();
        fs::write(d.join("Cargo.toml"), "").unwrap();
        fs::write(d.join("package.json"), "{}").unwrap();
        assert_eq!(detect(&d).kind, ProjectKind::Rust);
    }

    #[test]
    fn php_layout_skips_serve_and_edits_at_root() {
        let d = tmp();
        fs::create_dir_all(d.join("www")).unwrap();
        fs::write(d.join("www/composer.json"), "{}").unwrap();
        fs::write(d.join("www/artisan"), "").unwrap();
        let det = detect(&d);
        let yaml = windows_yaml_for(&det, &d).unwrap();
        assert!(
            !yaml.contains("php artisan serve"),
            "yaml should not contain serve:\n{yaml}"
        );
        assert!(
            yaml.contains("${EDITOR:-vi} ."),
            "edit should target root:\n{yaml}"
        );
        assert!(!yaml.contains("${EDITOR:-vi} www"));
    }

    #[test]
    fn node_dev_script_picked() {
        let d = tmp();
        fs::write(
            d.join("package.json"),
            r#"{"scripts": {"dev": "vite", "build": "vite build"}}"#,
        )
        .unwrap();
        assert_eq!(node_dev_command(&d).as_deref(), Some("npm run dev"));
    }

    #[test]
    fn node_start_fallback() {
        let d = tmp();
        fs::write(
            d.join("package.json"),
            r#"{"scripts": {"start": "node index.js"}}"#,
        )
        .unwrap();
        assert_eq!(node_dev_command(&d).as_deref(), Some("npm start"));
    }

    #[test]
    fn node_no_runnable_script() {
        let d = tmp();
        fs::write(d.join("package.json"), r#"{"scripts": {"build": "x"}}"#).unwrap();
        assert_eq!(node_dev_command(&d), None);
    }
}
