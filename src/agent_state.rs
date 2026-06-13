//! Agent state model for the agent-mode dashboard.
//!
//! Agents run in ordinary tmux panes — any session, any window. A pane is
//! tracked when it carries agent state, written as pane-scoped options:
//!
//! - `@sx-agent-state` — last-reported state (see [`AgentState`]). Written by
//!   native integrations (Claude Code hooks → `sessionx agent-state <state>`,
//!   wired by `sessionx agent-hooks install`); empty for agents with no hook.
//! - `@sx-agent-seen` — "1" once the user has focused the agent since it last
//!   finished, flipping `Done` → `Idle`.
//!
//! Panes without options are still picked up when their foreground process
//! looks like a known agent CLI (see [`is_agent_command`]).
//!
//! State is resolved in two tiers:
//!
//!   1. Native: trust `@sx-agent-state` when an integration set it.
//!   2. Generic: if the pane's process is back at a shell the agent has exited
//!      (`Done`); otherwise scrape the pane tail for an approval/prompt to tell
//!      `Blocked` from `Working`.
//!
//! The generic tier always wins on the "back at shell" signal so a stale native
//! state (e.g. the agent crashed mid-`Working`) can't get stuck.

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
    /// Emoji glyph for desktop notifications and the outer terminal title
    /// (OSC 9 / OSC 2), where color comes from the glyph itself.
    pub fn glyph(self) -> &'static str {
        match self {
            AgentState::Blocked => "🔴",
            AgentState::Working => "🟡",
            AgentState::Done => "🔵",
            AgentState::Idle => "🟢",
            AgentState::Unknown => "⚪",
        }
    }

    /// Single-width status dot for the TUI, colored separately via ratatui
    /// (see `dash::state_color`) so it stays crisp and unobtrusive.
    pub fn dot(self) -> &'static str {
        match self {
            AgentState::Unknown => "○",
            _ => "●",
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

/// True when `cmd` (a `pane_current_command` value) is a plain shell — i.e.
/// no agent process is in the foreground.
pub fn at_shell(cmd: &str) -> bool {
    SHELLS.iter().any(|s| cmd.eq_ignore_ascii_case(s))
}

/// Foreground process names that mark a pane as an agent even without any
/// `@sx-agent-state` option (agents with no hook integration).
const AGENT_COMMANDS: &[&str] = &[
    "claude",
    "codex",
    "aider",
    "gemini",
    "goose",
    "amp",
    "opencode",
    "droid",
    "cursor-agent",
];

/// True when `cmd` looks like a known agent CLI, or matches the first word of
/// the user's configured agent command.
///
/// Claude Code's native binary names its process after its own version
/// (`pane_current_command` = e.g. `2.1.174`), so a bare `x.y.z` process name
/// is treated as an agent too.
pub fn is_agent_command(cmd: &str, configured_agent: &str) -> bool {
    if AGENT_COMMANDS.iter().any(|a| cmd.eq_ignore_ascii_case(a)) {
        return true;
    }
    if looks_like_version(cmd) {
        return true;
    }
    match configured_agent.split_whitespace().next() {
        Some(first) if !at_shell(first) && !first.starts_with('$') && first != "exec" => {
            cmd.eq_ignore_ascii_case(first)
        }
        _ => false,
    }
}

/// `2.1.174` → true. Two or more dot-separated all-digit segments.
fn looks_like_version(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() >= 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

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
    let mut st = AgentState::parse(raw);

    if at_shell(current_cmd) {
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
    fn version_process_name_is_agent() {
        // Claude Code's native binary
        assert!(is_agent_command("2.1.174", "exec $SHELL"));
        assert!(is_agent_command("claude", "exec $SHELL"));
        assert!(is_agent_command("codex", "exec $SHELL"));
        assert!(!is_agent_command("zsh", "exec $SHELL"));
        assert!(!is_agent_command("node", "exec $SHELL"));
        assert!(!is_agent_command("1.x.2", "exec $SHELL"));
        // configured agent's first word matches
        assert!(is_agent_command("myagent", "myagent --flag"));
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
