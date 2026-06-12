//! `sessionx agent-state <state>` — report an agent's status to the dashboard.
//!
//! Meant to be called from inside an agent's own pane (e.g. from Claude Code
//! hooks), where `$TMUX_PANE` identifies the pane. Writes the pane-scoped
//! `@sx-agent-state` option that the agent-mode sidebar reads. Pass `--pane` to
//! target a specific pane instead of `$TMUX_PANE`.

use anyhow::{anyhow, Result};

use crate::agent_state::AgentState;
use crate::notify;
use crate::tmux;

pub fn run(state: &str, pane: Option<&str>) -> Result<()> {
    let parsed = AgentState::parse(state);
    if parsed == AgentState::Unknown {
        return Err(anyhow!(
            "unknown state '{state}' (try: blocked | working | done | idle)"
        ));
    }

    let pane = match pane {
        Some(p) => p.to_string(),
        None => std::env::var("TMUX_PANE").map_err(|_| {
            anyhow!("not inside a tmux pane ($TMUX_PANE unset); pass --pane <id>")
        })?,
    };

    let prev = AgentState::parse(&tmux::get_pane_option(&pane, "sx-agent-state"));
    tmux::set_pane_option(&pane, "sx-agent-state", parsed.as_str())?;
    // A fresh non-idle report means the agent has new activity to review.
    if parsed != AgentState::Idle {
        tmux::set_pane_option(&pane, "sx-agent-seen", "0")?;
    }

    // Surface state transitions: every change updates the tab-title glyph;
    // blocked/done additionally raise a desktop notification + bell.
    if parsed != prev {
        let (session, window) = tmux::pane_location(&pane).unwrap_or_default();
        // Agent-mode panes carry a handle; plain sessions fall back to a
        // human-readable location instead of the raw pane id.
        let handle = match tmux::get_pane_option(&pane, "sx-agent-handle") {
            h if h.is_empty() => format!("{session}:{window}"),
            h => h,
        };
        notify::agent_event(&session, &handle, parsed);
    }
    Ok(())
}
