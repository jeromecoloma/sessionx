//! Agent state model for the agent-mode dashboard.
//!
//! Each agent runs in its own pane inside the `sessionx-agentmode` session and
//! carries pane-scoped options describing its status:
//!
//!   - `@sx-agent`        — "1" marks the pane as an agent (vs. sidebar / parking).
//!   - `@sx-agent-handle` — the display name shown in the sidebar.
//!   - `@sx-agent-state`  — last-reported state (see [`AgentState`]). Written by
//!                          native integrations (Claude Code hooks → `sessionx
//!                          agent-state <state>`); empty for agents with no hook.
//!   - `@sx-agent-seen`   — "1" once the user has focused the agent since it last
//!                          finished, flipping `Done` → `Idle`.
//!
//! State is resolved in two tiers, mirroring herdr:
//!
//!   1. Native: trust `@sx-agent-state` when an integration set it.
//!   2. Generic: if the pane's process is back at a shell the agent has exited
//!      (`Done`); otherwise scrape the pane tail for an approval/prompt to tell
//!      `Blocked` from `Working`.
//!
//! The generic tier always wins on the "back at shell" signal so a stale native
//! state (e.g. the agent crashed mid-`Working`) can't get stuck.

/// Fixed name of the agent-mode dashboard session.
pub const SESSION: &str = "sessionx-agentmode";
/// Name of the window holding the sidebar + stage split.
pub const CONTROL_WINDOW: &str = "control";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Needs input/approval.
    Blocked,
    /// Actively running.
    Working,
    /// Finished, not yet reviewed.
    Done,
    /// Finished and reviewed (focused since finishing).
    Idle,
    /// No signal yet.
    Unknown,
}

impl AgentState {
    pub fn glyph(self) -> &'static str {
        match self {
            AgentState::Blocked => "🔴",
            AgentState::Working => "🟡",
            AgentState::Done => "🔵",
            AgentState::Idle => "🟢",
            AgentState::Unknown => "⚪",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AgentState::Blocked => "blocked",
            AgentState::Working => "working",
            AgentState::Done => "done",
            AgentState::Idle => "idle",
            AgentState::Unknown => "unknown",
        }
    }

    /// Canonical string written to `@sx-agent-state` by the hook writer.
    pub fn as_str(self) -> &'static str {
        self.label()
    }

    /// Parse a `@sx-agent-state` value; unknown/empty → [`AgentState::Unknown`].
    pub fn parse(s: &str) -> AgentState {
        match s.trim() {
            "blocked" => AgentState::Blocked,
            "working" => AgentState::Working,
            "done" => AgentState::Done,
            "idle" => AgentState::Idle,
            _ => AgentState::Unknown,
        }
    }
}

const SHELLS: &[&str] = &["bash", "zsh", "fish", "sh", "dash", "ksh", "tcsh"];

/// Substrings that suggest an agent is waiting on the user (approval/prompt).
const BLOCKED_HINTS: &[&str] = &[
    "do you want",
    "(y/n)",
    "[y/n]",
    "yes/no",
    "approve",
    "allow?",
    "permission",
    "press enter",
    "continue?",
    "❯ 1.",
    "1. yes",
    "waiting for",
];

/// Resolve an agent's effective state from its raw native value, seen flag,
/// foreground process name, and a tail of its pane output.
pub fn resolve(raw: &str, seen: bool, current_cmd: &str, tail: &str) -> AgentState {
    let at_shell = SHELLS.iter().any(|s| current_cmd.eq_ignore_ascii_case(s));

    let mut st = AgentState::parse(raw);

    if at_shell {
        // Process exited / dropped back to a prompt — authoritative "finished".
        st = AgentState::Done;
    } else if st == AgentState::Unknown {
        // No native signal but the agent process is running: distinguish
        // blocked-on-input from actively-working heuristically.
        let lower = tail.to_lowercase();
        st = if BLOCKED_HINTS.iter().any(|h| lower.contains(h)) {
            AgentState::Blocked
        } else {
            AgentState::Working
        };
    }

    if st == AgentState::Done && seen {
        AgentState::Idle
    } else {
        st
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_process_means_done() {
        assert_eq!(resolve("working", false, "zsh", ""), AgentState::Done);
    }

    #[test]
    fn done_and_seen_becomes_idle() {
        assert_eq!(resolve("done", true, "node", ""), AgentState::Idle);
        assert_eq!(resolve("", true, "bash", ""), AgentState::Idle);
    }

    #[test]
    fn native_state_trusted_while_running() {
        assert_eq!(resolve("blocked", false, "node", ""), AgentState::Blocked);
        assert_eq!(resolve("working", false, "python", ""), AgentState::Working);
    }

    #[test]
    fn generic_detects_blocked_from_tail() {
        assert_eq!(
            resolve("", false, "node", "Do you want to proceed? (y/n)"),
            AgentState::Blocked
        );
        assert_eq!(
            resolve("", false, "node", "compiling..."),
            AgentState::Working
        );
    }
}
