//! `sessionx mode agent` — the agent-mode dashboard.
//!
//! Builds (or re-attaches to) the `sessionx-agentmode` session. Its `control`
//! window is a two-pane split:
//!
//!     ┌────────────┬────────────────────────────┐
//!     │  sidebar   │           stage            │
//!     │ (this TUI) │   (focused agent's pane)   │
//!     └────────────┴────────────────────────────┘
//!
//! The left pane runs `sessionx dash` (the navigator). The right "stage" pane
//! starts as a parking shell; focusing an agent in the sidebar `swap-pane`s that
//! agent into the stage slot. Each agent otherwise lives in its own hidden
//! window, so its process keeps running whether staged or not.

use anyhow::{anyhow, Result};

use crate::agent_state::{CONTROL_WINDOW, SESSION};
use crate::tmux;

pub fn run(what: &str) -> Result<()> {
    match what {
        "agent" => run_agent(),
        other => Err(anyhow!("unknown mode '{other}' (try: agent)")),
    }
}

fn run_agent() -> Result<()> {
    if tmux::in_sessionx() && std::env::var("SESSIONX_ALLOW_NESTED").is_err() {
        return Err(anyhow!(
            "already running inside a sessionx-attached tmux session; \
             set SESSIONX_ALLOW_NESTED=1 to override"
        ));
    }

    if tmux::has_session(SESSION) {
        tmux::attach_or_switch(SESSION)?;
        return Ok(());
    }

    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")));

    // control window, first pane → sidebar (runs this dashboard's navigator).
    let (_wid, sidebar) = tmux::new_session(SESSION, &cwd, Some(CONTROL_WINDOW))?;
    tmux::set_pane_option(&sidebar, "sx-sidebar", "1")?;

    // Split off the stage slot to the right (~78% wide). Starts as a parking
    // shell; agents swap into it on focus.
    let _stage = tmux::split_window(&sidebar, &cwd, true, Some(78), None)?;

    // Launch the navigator in the sidebar and keep focus there.
    tmux::send_keys(&sidebar, "exec sessionx dash")?;
    tmux::select_pane(&sidebar)?;

    // Mark the session so it is recognisable (kept out of the managed list so
    // it doesn't clutter `sessionx ls --all`).
    tmux::set_user_option(SESSION, "sessionx-agentmode", "1")?;

    tmux::attach_or_switch(SESSION)?;
    Ok(())
}
