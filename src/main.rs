mod agent;
mod config;
mod tmux;
mod status;
mod themes;
mod worktree;
mod hooks;
mod hooks_repo;
mod cmd;
mod picker;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sessionx", version, about = "Simple tmux session manager with optional worktree mode")]
struct Cli {
    #[arg(short, long, global = true, help = "Print tmux/git commands as they run")]
    verbose: bool,

    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write a starter .sessionx.yaml in the current directory.
    ///
    /// Runs an interactive wizard when stdin/stdout is a TTY (mode, worktree,
    /// theme, project-aware layout). Falls back to the static template under
    /// non-TTY (CI/pipes) or when --yes is given.
    Init {
        /// Skip prompts; use defaults (or detected values).
        #[arg(long)]
        yes: bool,
        /// Overwrite an existing .sessionx.yaml after backing it up to .sessionx.yaml.bak.
        #[arg(long)]
        force: bool,
        /// Pin the status-bar theme without prompting.
        #[arg(long)]
        theme: Option<String>,
        /// Pin mode without prompting: session | window.
        #[arg(long)]
        mode: Option<String>,
        /// Enable worktree mode with the given dir (e.g. --worktree .worktrees).
        #[arg(long)]
        worktree: Option<String>,
    },
    /// Open .sessionx.yaml in $VISUAL/$EDITOR
    Edit,
    /// Create (or attach to) a session. Worktree-mode also creates a git worktree.
    Add {
        name: String,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        no_attach: bool,
    },
    /// List sessions managed by sessionx in the current project
    Ls {
        /// Print only handles (one per line) — used by shell completions
        #[arg(long)]
        names_only: bool,
        /// List all sessionx-managed sessions globally (across projects)
        #[arg(long)]
        all: bool,
    },
    /// Attach to any sessionx-managed session globally (no .sessionx.yaml needed)
    Open {
        /// Full session name (use TAB completion). Omit to print the list.
        name: Option<String>,
        /// Print only names — used by shell completions
        #[arg(long)]
        names_only: bool,
    },
    /// Kill a session (and remove worktree, in worktree mode)
    Rm {
        name: String,
        #[arg(long)]
        force: bool,
    },
    /// Print shell completions to stdout (bash, zsh, fish)
    Completions {
        shell: String,
    },
    /// List built-in status-bar themes
    Themes,
    /// Manage the global config (`~/.config/sessionx/config.yaml`).
    ///
    /// No args → open in $VISUAL/$EDITOR. Use `path` to print location,
    /// `get [key]` to read, `set <key> <value>` to write.
    Config {
        /// `path`, `get`, `set`, or omit to open in editor.
        arg: Option<String>,
        /// Key for `get` / `set`.
        key: Option<String>,
        /// Value for `set`.
        value: Option<String>,
    },
    /// Manage stack-specific hook recipes (sessionx-hooks repo).
    ///
    /// No args → list recipes. Use `info`/`install`/`update`/`repo` for control.
    Hooks {
        /// `list` (default), `info`, `install`, `update`, or `repo`.
        arg: Option<String>,
        /// Recipe id for `info` / `install`.
        id: Option<String>,
    },
    /// Manage the project's status-bar theme.
    ///
    /// No args → list themes. Bare theme name (e.g. `sessionx theme dracula`) is
    /// shorthand for `sessionx theme set <name>`. Use `set`/`preview` explicitly
    /// for finer control.
    Theme {
        /// `set`, `preview`, or a theme name (shorthand for `set <name>`).
        arg: Option<String>,
        /// When arg is `set` or `preview`: the theme name.
        name: Option<String>,
        /// Skip live-apply, write YAML only (for `set`).
        #[arg(long)]
        no_apply: bool,
        /// Target tmux session instead of the current client's session.
        #[arg(long)]
        session: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.verbose {
        std::env::set_var("SX_VERBOSE", "1");
    }
    match cli.cmd {
        None => cmd::default::run(),
        Some(Cmd::Init { yes, force, theme, mode, worktree }) => {
            cmd::init::run(cmd::init::InitOpts { yes, force, theme, mode, worktree })
        }
        Some(Cmd::Edit) => cmd::edit::run(),
        Some(Cmd::Add { name, base, no_attach }) => cmd::add::run(&name, base.as_deref(), !no_attach),
        Some(Cmd::Ls { names_only, all }) => cmd::ls::run(names_only, all),
        Some(Cmd::Open { name, names_only }) => cmd::open::run(name.as_deref(), names_only),
        Some(Cmd::Rm { name, force }) => cmd::rm::run(&name, force),
        Some(Cmd::Completions { shell }) => print_completions(&shell),
        Some(Cmd::Themes) => {
            for t in themes::list() {
                println!("{t}");
            }
            Ok(())
        }
        Some(Cmd::Config { arg, key, value }) => {
            match arg.as_deref() {
                None => cmd::config::run_edit(),
                Some("path") => cmd::config::run_path(),
                Some("get") => cmd::config::run_get(key.as_deref()),
                Some("set") => {
                    let k = key.ok_or_else(|| anyhow!("config set: missing <key>"))?;
                    let v = value.ok_or_else(|| anyhow!("config set: missing <value>"))?;
                    cmd::config::run_set(&k, &v)
                }
                Some(other) => Err(anyhow!(
                    "unknown config subcommand '{other}' (try path|get|set or no arg)"
                )),
            }
        }
        Some(Cmd::Hooks { arg, id }) => {
            match arg.as_deref() {
                None | Some("list") => cmd::hooks::run_list(),
                Some("info") => {
                    let id = id.ok_or_else(|| anyhow!("hooks info: missing <id>"))?;
                    cmd::hooks::run_info(&id)
                }
                Some("install") => {
                    let id = id.ok_or_else(|| anyhow!("hooks install: missing <id>"))?;
                    cmd::hooks::run_install(&id)
                }
                Some("update") => cmd::hooks::run_update(),
                Some("repo") => cmd::hooks::run_repo(),
                Some(other) => Err(anyhow!(
                    "unknown hooks subcommand '{other}' (try list|info|install|update|repo)"
                )),
            }
        }
        Some(Cmd::Theme { arg, name, no_apply, session }) => {
            match arg.as_deref() {
                None => cmd::theme::run_list(),
                Some("set") => {
                    let n = name.ok_or_else(|| anyhow!("theme set: missing <name>"))?;
                    cmd::theme::run_set(&n, !no_apply, session.as_deref())
                }
                Some("preview") => {
                    let n = name.ok_or_else(|| anyhow!("theme preview: missing <name>"))?;
                    cmd::theme::run_preview(&n, session.as_deref())
                }
                Some(theme_name) => {
                    // Shorthand: `sessionx theme <name>` ≡ `sessionx theme set <name>`.
                    if name.is_some() {
                        return Err(anyhow!(
                            "unexpected extra arg; did you mean `sessionx theme set <name>`?"
                        ));
                    }
                    cmd::theme::run_set(theme_name, !no_apply, session.as_deref())
                }
            }
        }
    }
}

fn print_completions(shell: &str) -> Result<()> {
    let body = match shell {
        "zsh"  => include_str!("../completions/_sessionx"),
        "bash" => include_str!("../completions/sessionx.bash"),
        "fish" => include_str!("../completions/sessionx.fish"),
        other  => return Err(anyhow!("unsupported shell: {other} (try zsh|bash|fish)")),
    };
    print!("{body}");
    Ok(())
}
