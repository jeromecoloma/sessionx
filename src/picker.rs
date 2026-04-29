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
    if items.is_empty() || !is_tty() {
        return Ok(None);
    }
    if has_bin("fzf") {
        return select_fzf(title, items);
    }
    select_inquire(title, items)
}

fn select_fzf(title: &str, items: &[String]) -> Result<Option<usize>> {
    let mut child = Command::new("fzf")
        .args([
            "--prompt",
            &format!("{title}> "),
            "--height=40%",
            "--reverse",
            "--no-multi",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    {
        let stdin = child.stdin.as_mut().ok_or_else(|| anyhow!("fzf: stdin unavailable"))?;
        for item in items {
            writeln!(stdin, "{item}")?;
        }
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Ok(None);
    }
    let chosen = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(items.iter().position(|s| s == &chosen))
}

fn select_inquire(title: &str, items: &[String]) -> Result<Option<usize>> {
    let opts: Vec<String> = items.to_vec();
    match inquire::Select::new(title, opts.clone()).prompt() {
        Ok(choice) => Ok(opts.iter().position(|s| s == &choice)),
        Err(_) => Ok(None),
    }
}
