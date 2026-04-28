mod config;
mod tmux;
mod status;
mod worktree;
mod hooks;
mod cmd;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sessionx", version, about = "Simple tmux session manager with optional worktree mode")]
struct Cli {
    #[arg(short, long, global = true, help = "Print tmux/git commands as they run")]
    verbose: bool,

    #[command(subcommand)]
    cmd: Cmd,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.verbose {
        std::env::set_var("SX_VERBOSE", "1");
    }
    match cli.cmd {
        Cmd::Init => cmd::init::run(),
        Cmd::Edit => cmd::edit::run(),
        Cmd::Add { name, base, no_attach } => cmd::add::run(&name, base.as_deref(), !no_attach),
        Cmd::Ls { names_only, all } => cmd::ls::run(names_only, all),
        Cmd::Open { name, names_only } => cmd::open::run(name.as_deref(), names_only),
        Cmd::Rm { name, force } => cmd::rm::run(&name, force),
        Cmd::Completions { shell } => print_completions(&shell),
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
