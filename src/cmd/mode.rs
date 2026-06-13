//! `sessionx mode agent` — enter the agent-mode dashboard.
//!
//! Runs the attention-inbox TUI (see [`crate::cmd::dash`]) directly in the
//! current terminal. There is no dedicated session and nothing to build:
//! agents are ordinary panes in ordinary sessions; the dashboard is a live
//! view over them, inside or outside tmux.

use anyhow::{anyhow, Result};

use crate::cmd::dash;

pub fn run(what: &str) -> Result<()> {
    match what {
        "agent" => dash::run(),
        other => Err(anyhow!("unknown mode '{other}' (try: agent)")),
    }
}
