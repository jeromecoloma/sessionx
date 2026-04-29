use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::{cmd, config, picker, tmux};

const DEFAULT_HANDLE: &str = "main";

enum Action {
    Attach,
    Init,
    Open(String),
    PlainTmux,
    Quit,
}

pub fn run() -> Result<()> {
    if !picker::is_tty() {
        eprintln!("sessionx: no subcommand given. Run `sessionx --help` for usage.");
        std::process::exit(2);
    }

    let loaded = config::find_and_load().ok();
    let cwd = std::env::current_dir()?;
    let in_git = is_git_repo(&cwd);
    let managed = tmux::list_managed_sessions().unwrap_or_default();

    let mut labels: Vec<String> = vec![];
    let mut actions: Vec<Action> = vec![];

    if loaded.is_some() {
        labels.push(format!("Attach/create project session ({DEFAULT_HANDLE})"));
        actions.push(Action::Attach);
    } else if in_git {
        labels.push("Init .sessionx.yaml here".to_string());
        actions.push(Action::Init);
    }

    for m in &managed {
        labels.push(format!("Open managed session: {}  [{}]", m.name, m.project));
        actions.push(Action::Open(m.name.clone()));
    }

    labels.push("New plain tmux session".to_string());
    actions.push(Action::PlainTmux);

    labels.push("Quit".to_string());
    actions.push(Action::Quit);

    let Some(idx) = picker::select("sessionx", &labels)? else {
        return Ok(());
    };

    match &actions[idx] {
        Action::Attach => cmd::add::run(DEFAULT_HANDLE, None, true),
        Action::Init => cmd::init::run(),
        Action::Open(name) => cmd::open::run(Some(name), false),
        Action::PlainTmux => plain_tmux(&cwd),
        Action::Quit => Ok(()),
    }
}

fn is_git_repo(cwd: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn plain_tmux(cwd: &Path) -> Result<()> {
    let base = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("tmux");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut name = sanitize(&format!("tmux-{base}-{}", stamp % 100000));
    let mut suffix = 0u32;
    while tmux::has_session(&name) {
        suffix += 1;
        name = sanitize(&format!("tmux-{base}-{}-{suffix}", stamp % 100000));
    }
    tmux::new_session(&name, cwd, None)?;
    tmux::attach_or_switch(&name)
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if matches!(c, '.' | ':' | ' ' | '\t') { '_' } else { c })
        .collect()
}
