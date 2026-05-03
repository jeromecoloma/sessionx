use anyhow::{anyhow, Result};
use std::io::{IsTerminal, Write};
use std::process::{Command, Stdio};

pub fn is_tty() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn has_bin(bin: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {bin}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Show an interactive picker over `items` and return the chosen index.
/// Returns `Ok(None)` if the user cancels or no TTY is attached.
/// Prefers `fzf` when available, falls back to `inquire`.
pub fn select(title: &str, items: &[String]) -> Result<Option<usize>> {
    Ok(select_with_keys(title, items, &[], None)?.map(|(idx, _)| idx))
}

/// Like `select`, but also reports which "expect" key the user pressed (if any).
/// `expect_keys` are passed to fzf via `--expect`. On Enter the returned key is `None`.
/// `header` is shown above the list (fzf only).
/// The inquire fallback ignores expect keys and always returns `None` for the key.
pub fn select_with_keys(
    title: &str,
    items: &[String],
    expect_keys: &[&str],
    header: Option<&str>,
) -> Result<Option<(usize, Option<String>)>> {
    if items.is_empty() || !is_tty() {
        return Ok(None);
    }
    if has_bin("fzf") {
        return select_fzf(title, items, expect_keys, header);
    }
    select_inquire(title, items)
}

fn select_fzf(
    title: &str,
    items: &[String],
    expect_keys: &[&str],
    header: Option<&str>,
) -> Result<Option<(usize, Option<String>)>> {
    let mut cmd = Command::new("fzf");
    cmd.args([
        "--prompt",
        &format!("{title}> "),
        "--height=40%",
        "--reverse",
        "--no-multi",
    ]);
    if !expect_keys.is_empty() {
        cmd.args(["--expect", &expect_keys.join(",")]);
    }
    if let Some(h) = header {
        cmd.args(["--header", h]);
    }
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("fzf: stdin unavailable"))?;
        for item in items {
            writeln!(stdin, "{item}")?;
        }
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let (key, chosen) = if expect_keys.is_empty() {
        (None, stdout.trim().to_string())
    } else {
        let mut lines = stdout.lines();
        let key_line = lines.next().unwrap_or("").trim().to_string();
        let chosen_line = lines.next().unwrap_or("").trim().to_string();
        let key = if key_line.is_empty() {
            None
        } else {
            Some(key_line)
        };
        (key, chosen_line)
    };
    if chosen.is_empty() {
        return Ok(None);
    }
    Ok(items.iter().position(|s| s == &chosen).map(|i| (i, key)))
}

fn select_inquire(title: &str, items: &[String]) -> Result<Option<(usize, Option<String>)>> {
    let opts: Vec<String> = items.to_vec();
    match inquire::Select::new(title, opts.clone()).prompt() {
        Ok(choice) => Ok(opts.iter().position(|s| s == &choice).map(|i| (i, None))),
        Err(_) => Ok(None),
    }
}

/// Yes/No confirmation prompt. Returns `Ok(false)` on cancel or no TTY.
pub fn confirm(message: &str, default: bool) -> Result<bool> {
    if !is_tty() {
        return Ok(false);
    }
    match inquire::Confirm::new(message)
        .with_default(default)
        .prompt()
    {
        Ok(b) => Ok(b),
        Err(_) => Ok(false),
    }
}

/// If `name` exceeds `max_chars`, offer a rename. Returns the chosen name —
/// either the user's input (sanitized via `sanitize`) or the original on
/// blank/cancel/empty/collision (via `is_taken`).
pub fn maybe_rename_long(
    name: String,
    max_chars: usize,
    sanitize: impl Fn(&str) -> String,
    is_taken: impl Fn(&str) -> bool,
) -> Result<String> {
    if name.chars().count() <= max_chars {
        return Ok(name);
    }
    if !is_tty() {
        return Ok(name);
    }
    let help = format!(
        "current: {name} ({} chars) — esc to keep",
        name.chars().count()
    );
    let raw = match inquire::Text::new("rename session?")
        .with_default(&name)
        .with_help_message(&help)
        .prompt()
    {
        Ok(s) => s,
        Err(_) => return Ok(name),
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == name {
        return Ok(name);
    }
    let candidate = sanitize(trimmed);
    if candidate.is_empty() || is_taken(&candidate) {
        return Ok(name);
    }
    Ok(candidate)
}

/// Prompt the user for free-text input.
/// Returns `Ok(None)` if cancelled (esc/ctrl+c), no TTY, or input is blank.
pub fn prompt(title: &str) -> Result<Option<String>> {
    if !is_tty() {
        return Ok(None);
    }
    let mut p = inquire::Text::new(title);
    if let Some(validator) = nonblank_validator() {
        p = p.with_validator(validator);
    }
    match p.prompt() {
        Ok(s) => {
            let s = s.trim().to_string();
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
        Err(_) => Ok(None),
    }
}

fn nonblank_validator() -> Option<inquire::validator::ValueRequiredValidator> {
    Some(inquire::validator::ValueRequiredValidator::default())
}
