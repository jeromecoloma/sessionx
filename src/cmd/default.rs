use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::StatusSpec;
use crate::{cmd, config, picker, status, themes, tmux};
use tmux::ManagedSession;

const DEFAULT_HANDLE: &str = "main";
const ROOT_HANDLE: &str = "root";

enum Action {
    Attach,
    AttachRoot,
    AttachWorktree,
    Init,
    Open(String),
    OpenPlain(String),
    PlainTmux,
    Quit,
}

pub fn run() -> Result<()> {
    if !picker::is_tty() {
        eprintln!("sessionx: no subcommand given. Run `sessionx --help` for usage.");
        std::process::exit(2);
    }

    if tmux::in_sessionx() && std::env::var("SESSIONX_ALLOW_NESTED").is_err() {
        eprintln!(
            "sessionx: already running inside a sessionx-attached tmux session. \
             Set SESSIONX_ALLOW_NESTED=1 to override."
        );
        std::process::exit(2);
    }

    let loaded = config::find_and_load().ok();
    let cwd = std::env::current_dir()?;
    let in_git = is_git_repo(&cwd);
    let mut managed = tmux::list_managed_sessions().unwrap_or_default();

    // Group: current project's managed sessions first, then others.
    if let Some(l) = &loaded {
        let here = l.project_root.display().to_string();
        managed.sort_by_key(|m| if m.project == here { 0 } else { 1 });
    }

    let mut labels: Vec<String> = vec![];
    let mut actions: Vec<Action> = vec![];

    if let Some(l) = &loaded {
        if l.worktree_mode() {
            labels.push(format!(
                "Attach/create main project session ({ROOT_HANDLE})"
            ));
            actions.push(Action::AttachRoot);
            labels.push("Add new worktree session…".to_string());
            actions.push(Action::AttachWorktree);
        } else {
            labels.push(format!("Attach/create project session ({DEFAULT_HANDLE})"));
            actions.push(Action::Attach);
        }
    } else if in_git {
        labels.push("Init .sessionx.yaml here".to_string());
        actions.push(Action::Init);
    }

    let here = loaded.as_ref().map(|l| l.project_root.display().to_string());
    for m in &managed {
        let label = if here.as_deref() == Some(m.project.as_str()) {
            format!("Open managed session: {}", m.name)
        } else {
            format!("Open managed session: {}  [{}]", m.name, m.project)
        };
        labels.push(label);
        actions.push(Action::Open(m.name.clone()));
    }

    let unmanaged = tmux::list_unmanaged_sessions().unwrap_or_default();
    for name in &unmanaged {
        labels.push(format!("Attach plain tmux session: {name}"));
        actions.push(Action::OpenPlain(name.clone()));
    }

    labels.push("New plain tmux session".to_string());
    actions.push(Action::PlainTmux);

    labels.push("Quit".to_string());
    actions.push(Action::Quit);

    let (expect_keys, header): (&[&str], Option<&str>) =
        if managed.is_empty() && unmanaged.is_empty() {
            (&[], None)
        } else {
            (
                &["ctrl-x"],
                Some("enter: select  ·  ctrl-x: delete session"),
            )
        };
    let Some((idx, key)) = picker::select_with_keys("sessionx", &labels, expect_keys, header)?
    else {
        return Ok(());
    };

    if key.as_deref() == Some("ctrl-x") {
        match &actions[idx] {
            Action::Open(name) => {
                if let Some(m) = managed.iter().find(|m| &m.name == name) {
                    return delete_managed(m);
                }
            }
            Action::OpenPlain(name) => {
                return delete_plain(name);
            }
            _ => {}
        }
        eprintln!("sessionx: ctrl-x only deletes existing sessions");
        return Ok(());
    }

    match &actions[idx] {
        Action::Attach => cmd::add::run(DEFAULT_HANDLE, None, true, false),
        Action::AttachRoot => cmd::add::run(ROOT_HANDLE, None, true, true),
        Action::AttachWorktree => {
            let Some(handle) = picker::prompt("worktree handle (e.g. feat-x)")? else {
                return Ok(());
            };
            cmd::add::run(&handle, None, true, false)
        }
        Action::Init => cmd::init::run(cmd::init::InitOpts::default()),
        Action::Open(name) => cmd::open::run(Some(name), false),
        Action::OpenPlain(name) => tmux::attach_or_switch(name),
        Action::PlainTmux => plain_tmux(&cwd),
        Action::Quit => Ok(()),
    }
}

fn delete_plain(name: &str) -> Result<()> {
    let msg = format!("Kill plain tmux session '{name}'?");
    if !picker::confirm(&msg, false)? {
        return Ok(());
    }
    if tmux::has_session(name) {
        tmux::kill_session(name)?;
        println!("killed session {name}");
    }
    Ok(())
}

fn delete_managed(m: &ManagedSession) -> Result<()> {
    let msg = format!("Delete session '{}' ({})?", m.name, m.project);
    if !picker::confirm(&msg, false)? {
        return Ok(());
    }
    let project_path = Path::new(&m.project);
    let handle = if m.handle.is_empty() {
        DEFAULT_HANDLE
    } else {
        &m.handle
    };
    match config::load_from_dir(project_path) {
        Ok(loaded) => cmd::rm::run_with_loaded(&loaded, handle, false),
        Err(_) => {
            if tmux::has_session(&m.name) {
                tmux::kill_session(&m.name)?;
                println!("killed session {}", m.name);
            }
            eprintln!(
                "sessionx: project config not found at {} — worktree cleanup skipped",
                m.project
            );
            Ok(())
        }
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
    let Some(theme) = pick_plain_theme()? else {
        return Ok(());
    };

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

    name = picker::maybe_rename_long(name, 20, sanitize, tmux::has_session)?;

    tmux::new_session(&name, cwd, None)?;

    if let Some(theme_name) = theme {
        let spec = StatusSpec {
            enabled: true,
            theme: Some(theme_name),
            ..StatusSpec::default()
        };
        status::apply(&name, &spec)?;
    }

    tmux::attach_or_switch(&name)
}

/// Prompt for a theme to apply to a plain tmux session.
/// Outer `Option` distinguishes cancel (`None` → abort spawn) from a real selection.
/// Inner `Option<String>` carries the theme name, or `None` for the "(no theme)" entry.
fn pick_plain_theme() -> Result<Option<Option<String>>> {
    let none_label = "(no theme)".to_string();
    let mut items = vec![none_label];
    items.extend(themes::list().iter().map(|s| s.to_string()));

    let Some(idx) = picker::select("theme for plain tmux session", &items)? else {
        return Ok(None);
    };
    if idx == 0 {
        Ok(Some(None))
    } else {
        Ok(Some(Some(items[idx].clone())))
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if matches!(c, '.' | ':' | ' ' | '\t') {
                '_'
            } else {
                c
            }
        })
        .collect()
}
