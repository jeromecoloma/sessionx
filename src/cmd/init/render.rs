/// Resolved choices that drive template rendering.
pub struct Resolved {
    pub mode: String,
    pub theme: String,
    pub worktree_dir: Option<String>,
    pub windows_yaml: Option<String>,
    /// When `Some((post, pre))`, uncomment the post_create / pre_remove block
    /// in the template and replace it with the given bash commands.
    pub hook_commands: Option<(String, String)>,
}

pub fn apply(template: &str, r: &Resolved) -> String {
    let mut out = template.to_string();
    out = replace_mode(&out, &r.mode);
    out = replace_theme(&out, &r.theme);
    if let Some(dir) = &r.worktree_dir {
        out = enable_worktree(&out, dir);
    }
    if let Some(wy) = &r.windows_yaml {
        out = replace_windows(&out, wy);
    }
    if let Some((post, pre)) = &r.hook_commands {
        out = enable_post_create(&out, post, pre);
    }
    out
}

/// Replace the commented `# post_create:` / `# pre_remove:` block in the template
/// with uncommented hooks running the given bash commands.
fn enable_post_create(input: &str, post_cmd: &str, pre_cmd: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim() == "# post_create:") else {
        return input.to_string();
    };
    // Block runs through the `# pre_remove:` line and its single sub-line.
    let mut end = start + 1;
    let mut saw_pre_remove = false;
    for (i, l) in lines.iter().enumerate().skip(start + 1) {
        let t = l.trim();
        if t == "# pre_remove:" {
            saw_pre_remove = true;
            end = i + 1;
            continue;
        }
        if saw_pre_remove {
            if t.starts_with("#   ") {
                end = i + 1;
                break;
            }
            break;
        }
        if !t.starts_with("#") || t.is_empty() {
            break;
        }
        end = i + 1;
    }

    let replacement = format!(
        "post_create:\n  - {post_cmd}\npre_remove:\n  - {pre_cmd}"
    );

    let mut out = String::with_capacity(input.len());
    for l in &lines[..start] {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(&replacement);
    out.push('\n');
    for l in &lines[end..] {
        out.push_str(l);
        out.push('\n');
    }
    if !input.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn replace_mode(input: &str, mode: &str) -> String {
    rewrite_lines(input, |line| {
        if line.starts_with("mode:") {
            Some(format!("mode: {mode}"))
        } else {
            None
        }
    })
}

fn replace_theme(input: &str, theme: &str) -> String {
    rewrite_lines(input, |line| {
        let trimmed = line.trim_start();
        if !line.starts_with(' ') {
            return None;
        }
        if trimmed.starts_with("theme:") {
            let indent_len = line.len() - trimmed.len();
            let indent = &line[..indent_len];
            Some(format!("{indent}theme: {theme}"))
        } else {
            None
        }
    })
}

fn enable_worktree(input: &str, dir: &str) -> String {
    rewrite_lines(input, |line| {
        let t = line.trim_start();
        if t.starts_with("# worktree_dir:") {
            Some(format!("worktree_dir: {dir}"))
        } else {
            None
        }
    })
}

/// Replace the entire `windows:` top-level block with `windows:\n{body}`.
/// Ends at the next blank line OR the next top-level key/comment that follows the block.
fn replace_windows(input: &str, body: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let Some(start) = lines.iter().position(|l| *l == "windows:") else {
        return input.to_string();
    };

    let mut end = lines.len();
    for (i, l) in lines.iter().enumerate().skip(start + 1) {
        if l.is_empty() {
            end = i;
            break;
        }
        let is_indented = l.starts_with(' ') || l.starts_with('\t');
        if !is_indented {
            end = i;
            break;
        }
    }

    let mut out = String::with_capacity(input.len());
    for l in &lines[..start] {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str("windows:\n");
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
    for l in &lines[end..] {
        out.push_str(l);
        out.push('\n');
    }
    if !input.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn rewrite_lines<F>(input: &str, mut f: F) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    let mut out = String::with_capacity(input.len());
    let trailing_nl = input.ends_with('\n');
    let line_count = input.lines().count();
    for (i, line) in input.lines().enumerate() {
        match f(line) {
            Some(replacement) => out.push_str(&replacement),
            None => out.push_str(line),
        }
        if i + 1 < line_count || trailing_nl {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = include_str!("../../../templates/sessionx.yaml.tmpl");

    fn parses(yaml: &str) -> bool {
        serde_yaml::from_str::<crate::config::Config>(yaml).is_ok()
    }

    #[test]
    fn defaults_render_unchanged_semantically() {
        let r = Resolved {
            mode: "session".into(),
            theme: "tokyo-night".into(),
            worktree_dir: None,
            windows_yaml: None,
            hook_commands: None,
        };
        let out = apply(TEMPLATE, &r);
        assert!(parses(&out), "rendered yaml must parse");
        assert!(out.contains("mode: session"));
        assert!(out.contains("theme: tokyo-night"));
    }

    #[test]
    fn swaps_mode_and_theme() {
        let r = Resolved {
            mode: "window".into(),
            theme: "dracula".into(),
            worktree_dir: None,
            windows_yaml: None,
            hook_commands: None,
        };
        let out = apply(TEMPLATE, &r);
        assert!(parses(&out));
        assert!(out.contains("mode: window"));
        assert!(out.contains("theme: dracula"));
        assert!(!out.contains("mode: session"));
        assert!(!out.contains("theme: tokyo-night"));
    }

    #[test]
    fn enables_worktree() {
        let r = Resolved {
            mode: "session".into(),
            theme: "tokyo-night".into(),
            worktree_dir: Some(".worktrees".into()),
            windows_yaml: None,
            hook_commands: None,
        };
        let out = apply(TEMPLATE, &r);
        assert!(parses(&out));
        assert!(out.contains("\nworktree_dir: .worktrees\n"));
    }

    #[test]
    fn replaces_windows_block() {
        let body = "  - name: shell\n    panes:\n      - command: exec $SHELL\n        focus: true\n  - name: dev\n    panes:\n      - command: npm run dev\n";
        let r = Resolved {
            mode: "session".into(),
            theme: "tokyo-night".into(),
            worktree_dir: None,
            windows_yaml: Some(body.into()),
            hook_commands: None,
        };
        let out = apply(TEMPLATE, &r);
        assert!(parses(&out), "rendered:\n{out}");
        assert!(out.contains("- name: dev"));
        assert!(out.contains("npm run dev"));
        // Original `edit` window should be gone.
        let edit_count = out.matches("name: edit").count();
        assert_eq!(edit_count, 0);
    }

    #[test]
    fn enables_post_create_hooks() {
        let r = Resolved {
            mode: "session".into(),
            theme: "tokyo-night".into(),
            worktree_dir: None,
            windows_yaml: None,
            hook_commands: Some((
                "SX_LARAVEL_DIR=www bash ~/.sessionx/scripts/laravel-herd/setup.sh".into(),
                "SX_LARAVEL_DIR=www bash ~/.sessionx/scripts/laravel-herd/teardown.sh".into(),
            )),
        };
        let out = apply(TEMPLATE, &r);
        assert!(parses(&out), "rendered:\n{out}");
        assert!(out.contains("\npost_create:\n  - SX_LARAVEL_DIR=www bash ~/.sessionx/scripts/laravel-herd/setup.sh"));
        assert!(out.contains("\npre_remove:\n  - SX_LARAVEL_DIR=www bash ~/.sessionx/scripts/laravel-herd/teardown.sh"));
        assert!(!out.contains("# post_create:"));
    }

    #[test]
    fn theme_replacement_only_inside_status() {
        // Sanity: don't accidentally rewrite a top-level `theme:` (template has none, but be safe).
        let input = "theme: top-level-should-not-match\nstatus:\n  theme: tokyo-night\n";
        let out = replace_theme(input, "nord");
        assert!(out.contains("theme: top-level-should-not-match"));
        assert!(out.contains("  theme: nord"));
    }
}
