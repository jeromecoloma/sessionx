use anyhow::Result;

use super::detect::{Detected, ProjectKind};
use super::InitOpts;
use crate::agent;
use crate::hooks_repo::{self, RecipeMeta};
use crate::picker;
use crate::themes;

pub struct Choices {
    pub mode: String,
    pub theme: String,
    pub worktree_dir: Option<String>,
    pub use_detected_layout: bool,
    /// `Some(recipe)` if the user picked one in the wizard.
    pub recipe: Option<RecipeMeta>,
}

const BACK_LABEL: &str = "← Back";

enum Outcome<T> {
    Chose(T),
    Back,
    Cancel,
}

#[derive(Default)]
struct State {
    mode: Option<String>,
    worktree_dir: Option<Option<String>>,
    theme: Option<String>,
    use_detected_layout: Option<bool>,
    recipe: Option<Option<RecipeMeta>>,
}

#[derive(Clone, Copy)]
enum Step {
    Mode,
    Worktree,
    Theme,
    Layout,
    Recipe,
    Agent,
    Done,
}

impl Step {
    fn next(self) -> Self {
        match self {
            Self::Mode => Self::Worktree,
            Self::Worktree => Self::Theme,
            Self::Theme => Self::Layout,
            Self::Layout => Self::Recipe,
            Self::Recipe => Self::Agent,
            Self::Agent => Self::Done,
            Self::Done => Self::Done,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Mode | Self::Worktree => Self::Mode,
            Self::Theme => Self::Worktree,
            Self::Layout => Self::Theme,
            Self::Recipe => Self::Layout,
            Self::Agent => Self::Recipe,
            Self::Done => Self::Agent,
        }
    }
}

pub fn run(opts: &InitOpts, det: &Detected) -> Result<Option<Choices>> {
    let kind = det.kind;

    let mut s = State::default();
    if let Some(m) = &opts.mode {
        s.mode = Some(normalize_mode(m)?);
    }
    if let Some(d) = &opts.worktree {
        s.worktree_dir = Some(Some(d.clone()));
    }
    if let Some(t) = &opts.theme {
        themes::load(t)?;
        s.theme = Some(t.clone());
    }

    // Compute available recipes for this stack once, up-front. Failures (e.g.
    // offline) just disable the recipe step silently.
    let stack_recipes = available_recipes_for(kind);

    let mut step = Step::Mode;
    loop {
        if matches!(step, Step::Mode) && s.mode.is_some() {
            step = step.next();
            continue;
        }
        if matches!(step, Step::Worktree) && s.worktree_dir.is_some() {
            step = step.next();
            continue;
        }
        if matches!(step, Step::Theme) && s.theme.is_some() {
            step = step.next();
            continue;
        }
        if matches!(step, Step::Layout) && matches!(kind, ProjectKind::Generic) {
            s.use_detected_layout = Some(false);
            step = step.next();
            continue;
        }
        // Agent step is a one-time global preference. Skip it whenever the
        // user already has `agent:` set in ~/.config/sessionx/config.yaml.
        if matches!(step, Step::Agent) && agent::global_agent_set() {
            step = step.next();
            continue;
        }
        if matches!(step, Step::Recipe) {
            let worktree_on = matches!(s.worktree_dir, Some(Some(_)));
            let visible: Vec<RecipeMeta> = stack_recipes
                .iter()
                .filter(|r| !r.requires_worktree || worktree_on)
                .cloned()
                .collect();
            if visible.is_empty() {
                s.recipe = Some(None);
                step = step.next();
                continue;
            }
        }

        match step {
            Step::Mode => match prompt_mode(false)? {
                Outcome::Chose(v) => {
                    s.mode = Some(v);
                    step = step.next();
                }
                Outcome::Back => {}
                Outcome::Cancel => return Ok(None),
            },
            Step::Worktree => match prompt_worktree(true)? {
                Outcome::Chose(v) => {
                    s.worktree_dir = Some(v);
                    step = step.next();
                }
                Outcome::Back => {
                    s.mode = None;
                    step = step.prev();
                }
                Outcome::Cancel => return Ok(None),
            },
            Step::Theme => match prompt_theme(true)? {
                Outcome::Chose(v) => {
                    s.theme = Some(v);
                    step = step.next();
                }
                Outcome::Back => {
                    s.worktree_dir = None;
                    step = step.prev();
                }
                Outcome::Cancel => return Ok(None),
            },
            Step::Layout => match prompt_layout(kind, true)? {
                Outcome::Chose(v) => {
                    s.use_detected_layout = Some(v);
                    step = step.next();
                }
                Outcome::Back => {
                    s.theme = None;
                    step = step.prev();
                }
                Outcome::Cancel => return Ok(None),
            },
            Step::Recipe => {
                let worktree_on = matches!(s.worktree_dir, Some(Some(_)));
                let visible: Vec<RecipeMeta> = stack_recipes
                    .iter()
                    .filter(|r| !r.requires_worktree || worktree_on)
                    .cloned()
                    .collect();
                match prompt_recipe(&visible, true)? {
                    Outcome::Chose(v) => {
                        s.recipe = Some(v);
                        step = step.next();
                    }
                    Outcome::Back => {
                        s.use_detected_layout = None;
                        step = step.prev();
                    }
                    Outcome::Cancel => return Ok(None),
                }
            }
            Step::Agent => match prompt_agent(true)? {
                Outcome::Chose(choice) => {
                    if let Some(name) = choice {
                        agent::save_global_agent(&name)?;
                    }
                    step = step.next();
                }
                Outcome::Back => {
                    s.recipe = None;
                    step = step.prev();
                }
                Outcome::Cancel => return Ok(None),
            },
            Step::Done => break,
        }
    }

    Ok(Some(Choices {
        mode: s.mode.unwrap(),
        theme: s.theme.unwrap(),
        worktree_dir: s.worktree_dir.unwrap_or(None),
        use_detected_layout: s.use_detected_layout.unwrap_or(false),
        recipe: s.recipe.unwrap_or(None),
    }))
}

/// Try to fetch the manifest and filter recipes by the detected stack.
/// Returns an empty Vec if anything fails (offline-friendly).
fn available_recipes_for(kind: ProjectKind) -> Vec<RecipeMeta> {
    let stack = match kind {
        ProjectKind::Php => "php",
        ProjectKind::Node => "node",
        ProjectKind::Rust => "rust",
        ProjectKind::Python => "python",
        ProjectKind::Generic => return vec![],
    };
    let cache = match hooks_repo::ensure_cloned() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let m = match hooks_repo::read_manifest(&cache) {
        Ok(m) => m,
        Err(_) => return vec![],
    };
    m.recipes.into_iter().filter(|r| r.stack == stack).collect()
}

fn normalize_mode(m: &str) -> Result<String> {
    match m {
        "session" | "window" => Ok(m.to_string()),
        other => Err(anyhow::anyhow!(
            "invalid --mode '{other}' (expected 'session' or 'window')"
        )),
    }
}

fn pick(title: &str, mut items: Vec<String>, allow_back: bool) -> Result<Outcome<usize>> {
    let back_idx = if allow_back {
        items.push(BACK_LABEL.to_string());
        Some(items.len() - 1)
    } else {
        None
    };
    let Some(idx) = picker::select(title, &items)? else {
        return Ok(Outcome::Cancel);
    };
    if Some(idx) == back_idx {
        return Ok(Outcome::Back);
    }
    Ok(Outcome::Chose(idx))
}

fn prompt_mode(allow_back: bool) -> Result<Outcome<String>> {
    let items = vec![
        "session — each `add` creates a tmux session (default)".to_string(),
        "window — each `add` creates a window in the current session".to_string(),
    ];
    Ok(match pick("mode", items, allow_back)? {
        Outcome::Chose(0) => Outcome::Chose("session".into()),
        Outcome::Chose(_) => Outcome::Chose("window".into()),
        Outcome::Back => Outcome::Back,
        Outcome::Cancel => Outcome::Cancel,
    })
}

fn prompt_worktree(allow_back: bool) -> Result<Outcome<Option<String>>> {
    let items = vec![
        "no — keep it simple (default)".to_string(),
        "yes — use git worktrees in .worktrees/".to_string(),
    ];
    Ok(match pick("enable worktree mode?", items, allow_back)? {
        Outcome::Chose(1) => Outcome::Chose(Some(".worktrees".into())),
        Outcome::Chose(_) => Outcome::Chose(None),
        Outcome::Back => Outcome::Back,
        Outcome::Cancel => Outcome::Cancel,
    })
}

fn prompt_theme(allow_back: bool) -> Result<Outcome<String>> {
    let names: Vec<String> = themes::list().iter().map(|s| (*s).to_string()).collect();
    let chosen = pick("status-bar theme", names.clone(), allow_back)?;
    Ok(match chosen {
        Outcome::Chose(i) => Outcome::Chose(names[i].clone()),
        Outcome::Back => Outcome::Back,
        Outcome::Cancel => Outcome::Cancel,
    })
}

fn prompt_layout(kind: ProjectKind, allow_back: bool) -> Result<Outcome<bool>> {
    let items = vec![
        format!("yes — use suggested {} layout", kind.label()),
        "no — use the generic 2-window default".to_string(),
    ];
    Ok(
        match pick(
            &format!("detected {} project, use suggested layout?", kind.label()),
            items,
            allow_back,
        )? {
            Outcome::Chose(0) => Outcome::Chose(true),
            Outcome::Chose(_) => Outcome::Chose(false),
            Outcome::Back => Outcome::Back,
            Outcome::Cancel => Outcome::Cancel,
        },
    )
}

/// One-time global agent preference. `Ok(Outcome::Chose(None))` means "skip,
/// keep default". `Ok(Outcome::Chose(Some(name)))` means save `name`.
fn prompt_agent(allow_back: bool) -> Result<Outcome<Option<String>>> {
    let presets = ["claude", "codex", "aider", "gh copilot"];
    let mut items: Vec<String> = presets.iter().map(|s| s.to_string()).collect();
    items.push("custom… (type a command)".to_string());
    items.push("skip — don't set a default agent".to_string());

    let custom_idx = presets.len();
    let skip_idx = custom_idx + 1;

    Ok(
        match pick(
            "default AI agent for the `agent` window (saved to ~/.config/sessionx/config.yaml)",
            items,
            allow_back,
        )? {
            Outcome::Chose(i) if i == skip_idx => Outcome::Chose(None),
            Outcome::Chose(i) if i == custom_idx => {
                match picker::prompt("agent command (e.g. \"claude\", \"codex --interactive\")")? {
                    Some(s) => Outcome::Chose(Some(s)),
                    None => Outcome::Chose(None),
                }
            }
            Outcome::Chose(i) => Outcome::Chose(Some(presets[i].to_string())),
            Outcome::Back => Outcome::Back,
            Outcome::Cancel => Outcome::Cancel,
        },
    )
}

/// Recipe options: `None` (no hooks) or one of the available recipes.
fn prompt_recipe(recipes: &[RecipeMeta], allow_back: bool) -> Result<Outcome<Option<RecipeMeta>>> {
    let mut items = vec!["none — no hook recipe".to_string()];
    for r in recipes {
        items.push(format!("{:<16} — {}", r.id, r.description));
    }
    Ok(match pick("install a hook recipe?", items, allow_back)? {
        Outcome::Chose(0) => Outcome::Chose(None),
        Outcome::Chose(i) => Outcome::Chose(Some(recipes[i - 1].clone())),
        Outcome::Back => Outcome::Back,
        Outcome::Cancel => Outcome::Cancel,
    })
}
