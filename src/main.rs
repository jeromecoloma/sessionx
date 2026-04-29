mod config;
mod tmux;
mod status;
mod themes;
mod worktree;
mod hooks;
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
    /// Write a starter .sessionx.yaml in the current directory
    Init,
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
        Some(Cmd::Init) => cmd::init::run(),
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
