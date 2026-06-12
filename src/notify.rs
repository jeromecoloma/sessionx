//! Desktop notifications to the user's terminal via OSC 9.
//!
//! An escape sequence emitted inside a tmux pane doesn't reliably reach the
//! outer terminal (passthrough must be enabled and the pane visible), so we
//! write the sequence directly to each attached client's tty instead. OSC 9
//! is the most widely supported notification sequence (Ghostty, kitty,
//! iTerm2, WezTerm, foot); terminals raise a desktop notification when the
//! receiving surface is unfocused — so the user only gets pinged when they
//! aren't already looking at that tab. Terminals without OSC 9 ignore it and
//! fall back to the trailing bell.

use std::io::Write;

use crate::agent;
use crate::agent_state::AgentState;
use crate::tmux;

/// True unless notifications are switched off via `SX_NOTIFY=0` or
/// `notify: false` in the global config.
pub fn enabled() -> bool {
    if let Ok(v) = std::env::var("SX_NOTIFY") {
        let v = v.trim();
        if v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off") {
            return false;
        }
    }
    agent::global_notify_enabled()
}

/// Notify about an agent state change. Every transition updates the tab
/// title glyph; only `Blocked` and `Done` warrant interrupting the user
/// with a notification + bell.
pub fn agent_event(session: &str, handle: &str, state: AgentState) {
    if !enabled() {
        return;
    }
    set_title(session, &format!("{} {handle}", state.glyph()));
    let body = match state {
        AgentState::Blocked => format!("sessionx: {} {handle} needs your input", state.glyph()),
        AgentState::Done => format!("sessionx: {} {handle} finished", state.glyph()),
        _ => return,
    };
    send(session, &body);
}

/// Emit an OSC 9 notification on every tmux client attached to `session`,
/// falling back to all clients when none are (the user is attached elsewhere
/// — they still want to hear about it). Best-effort: failures are ignored.
///
/// A BEL follows the OSC (unless `bell: false`): terminals without OSC 9
/// support ignore the sequence silently, so the bell is what reaches them —
/// sound, dock bounce, or tab badge depending on the terminal.
pub fn send(session: &str, body: &str) {
    let mut seq = format!("\x1b]9;{}\x1b\\", clean(body));
    if agent::global_bell_enabled() {
        seq.push('\x07');
    }
    write_clients(session, &seq);
}

/// Set the outer terminal's title (OSC 2) on every client attached to
/// `session`. Persists in the tab bar until the next title write — tmux
/// leaves the outer title alone unless `set-titles` is on — so the glyph
/// shows the agent's last reported state at a glance.
pub fn set_title(session: &str, text: &str) {
    if !agent::global_title_enabled() {
        return;
    }
    write_clients(session, &format!("\x1b]2;{}\x1b\\", clean(text)));
}

/// Write a raw sequence to every tmux client tty attached to `session`,
/// falling back to all clients when none are.
fn write_clients(session: &str, seq: &str) {
    let mut ttys = tmux::client_ttys(Some(session));
    if ttys.is_empty() {
        ttys = tmux::client_ttys(None);
    }
    for tty in ttys {
        if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(&tty) {
            let _ = f.write_all(seq.as_bytes());
        }
    }
}

/// Strip control characters and the OSC field separator so user-supplied
/// handles can't break out of the sequence.
fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control() && *c != ';').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_strips_separators_and_controls() {
        assert_eq!(clean("my-agent"), "my-agent");
        assert_eq!(clean("a;b\x1b]c\x07"), "ab]c");
    }
}
