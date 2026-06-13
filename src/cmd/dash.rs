//! `sessionx mode agent` — the agent-mode dashboard (attention inbox).
//!
//! A full-screen TUI over every agent pane on the tmux server. Agents live in
//! ordinary sessions — nothing is hidden or swapped around. The list is an
//! inbox sorted by urgency:
//!
//!     needs you   (blocked — waiting on approval/input)
//!     running     (working)
//!     done        (finished, not yet reviewed)
//!     idle        (finished and reviewed)
//!
//! The right panel shows a live tail of the selected agent's pane. `Enter`
//! jumps to the agent: `switch-client` when the dashboard runs inside tmux,
//! suspend-and-attach when outside. Jumping marks the agent seen.
//!
//! A pane is tracked when it carries `@sx-agent-state` (written by Claude Code
//! hooks via `sessionx agent-state`, wired with `sessionx agent-hooks install`)
//! or when its foreground process looks like a known agent CLI. Panes that
//! return to a plain shell after being reviewed are cleaned up and dropped.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use ansi_to_tui::IntoText;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::agent;
use crate::agent_state::{self, AgentState};
use crate::notify;
use crate::tmux;

/// Lines of pane tail fetched per agent for the generic blocked/working
/// heuristic (cheap, runs for every agent each poll).
const HEURISTIC_TAIL_LINES: u32 = 8;
/// Lines of pane tail fetched for the preview panel (selected agent only).
const PREVIEW_TAIL_LINES: u32 = 40;

struct AgentView {
    pane_id: String,
    session: String,
    window_id: String,
    label: String,
    state: AgentState,
}

enum Mode {
    Browse,
    ConfirmKill,
}

struct App {
    agents: Vec<AgentView>,
    sel: usize,
    mode: Mode,
    /// `$TMUX_PANE` when the dashboard itself runs inside tmux (excluded from
    /// the agent list; jumping uses switch-client instead of attach).
    own_pane: String,
    in_tmux: bool,
    configured_agent: String,
    preview: String,
    /// Horizontal scroll offset (columns) into the preview; reset when the
    /// selected agent changes.
    preview_hscroll: u16,
    /// Last resolved state per pane id, for transition detection.
    prev_states: HashMap<String, AgentState>,
    /// False until the first reload — suppresses a notification burst for
    /// agents that were already blocked/done when the dashboard started.
    primed: bool,
}

impl App {
    fn new() -> Self {
        App {
            agents: vec![],
            sel: 0,
            mode: Mode::Browse,
            own_pane: std::env::var("TMUX_PANE").unwrap_or_default(),
            in_tmux: tmux::in_tmux(),
            configured_agent: agent::resolve(),
            preview: String::new(),
            preview_hscroll: 0,
            prev_states: HashMap::new(),
            primed: false,
        }
    }

    /// Re-scan the server's panes and recompute each agent's state.
    fn reload(&mut self) -> Result<()> {
        let selected_pane = self.selected().map(|a| a.pane_id.clone());
        let panes = tmux::list_all_panes()?;
        let mut views: Vec<AgentView> = vec![];
        for p in &panes {
            if p.id == self.own_pane {
                continue;
            }
            let tracked = !p.state_raw.is_empty()
                || agent_state::is_agent_command(&p.current_cmd, &self.configured_agent);
            if !tracked {
                continue;
            }

            // Reviewed agents whose pane is back at a plain shell are spent:
            // clear the options so the pane stops being tracked.
            if p.seen && !p.state_raw.is_empty() && agent_state::at_shell(&p.current_cmd) {
                tmux::unset_pane_option(&p.id, "sx-agent-state");
                tmux::unset_pane_option(&p.id, "sx-agent-seen");
                self.prev_states.remove(&p.id);
                continue;
            }

            let tail = tmux::capture_pane_tail(&p.id, HEURISTIC_TAIL_LINES).unwrap_or_default();
            let state = agent_state::resolve(&p.state_raw, p.seen, &p.current_cmd, &tail);
            let label = format!("{} · {}", p.session, p.window_name);

            // Notify on transitions into blocked/done, but only when the
            // signal came from the generic tier — natively reported states
            // already notified via `sessionx agent-state`.
            let transition = self.prev_states.get(&p.id) != Some(&state);
            let natively_reported = AgentState::parse(&p.state_raw) == state;
            if self.primed
                && transition
                && !natively_reported
                && matches!(state, AgentState::Blocked | AgentState::Done)
            {
                notify::agent_event(&p.session, &label, state);
            }

            views.push(AgentView {
                pane_id: p.id.clone(),
                session: p.session.clone(),
                window_id: p.window_id.clone(),
                label,
                state,
            });
        }
        views.sort_by_key(|v| (urgency(v.state), v.label.clone()));
        self.prev_states = views.iter().map(|v| (v.pane_id.clone(), v.state)).collect();
        self.primed = true;
        self.agents = views;

        // Keep the cursor on the same agent across re-sorts.
        if let Some(pid) = selected_pane {
            if let Some(i) = self.agents.iter().position(|a| a.pane_id == pid) {
                self.sel = i;
            }
        }
        if self.sel >= self.agents.len() {
            self.sel = self.agents.len().saturating_sub(1);
        }

        self.preview = match self.selected() {
            Some(a) => tmux::capture_pane_ansi(&a.pane_id, PREVIEW_TAIL_LINES)
                .unwrap_or_default()
                .trim_end()
                .to_string(),
            None => String::new(),
        };
        Ok(())
    }

    fn selected(&self) -> Option<&AgentView> {
        self.agents.get(self.sel)
    }

    fn next(&mut self) {
        if !self.agents.is_empty() {
            self.sel = (self.sel + 1) % self.agents.len();
            self.preview_hscroll = 0;
        }
    }

    fn prev(&mut self) {
        if !self.agents.is_empty() {
            self.sel = (self.sel + self.agents.len() - 1) % self.agents.len();
            self.preview_hscroll = 0;
        }
    }

    /// Widest line in the current preview (in columns).
    fn preview_max_width(&self) -> usize {
        self.preview
            .lines()
            .map(|l| {
                // Strip ANSI escapes before measuring display width.
                l.into_text()
                    .map(|t| t.width())
                    .unwrap_or_else(|_| l.chars().count())
            })
            .max()
            .unwrap_or(0)
    }

    fn scroll_left(&mut self) {
        self.preview_hscroll = self.preview_hscroll.saturating_sub(8);
    }

    fn scroll_right(&mut self) {
        self.preview_hscroll = self.preview_hscroll.saturating_add(8);
    }

    /// Mark the selected agent seen and focus its pane. Inside tmux this
    /// switches the client; outside it suspends the TUI and attaches.
    /// Returns true when the caller must re-init the terminal (attach path).
    fn jump_selected(&mut self) -> Result<bool> {
        let Some(a) = self.selected() else {
            return Ok(false);
        };
        let (pane, session, window) = (a.pane_id.clone(), a.session.clone(), a.window_id.clone());
        tmux::set_pane_option(&pane, "sx-agent-seen", "1")?;
        if self.in_tmux {
            tmux::focus_pane(&session, &window, &pane)?;
            Ok(false)
        } else {
            ratatui::restore();
            let res = tmux::attach_at(&session, &window, &pane);
            // Back from the attach (user detached) — resume the dashboard.
            res.map(|_| true)
        }
    }

    fn kill_selected(&mut self) -> Result<()> {
        if let Some(a) = self.selected() {
            tmux::kill_pane(&a.pane_id)?;
        }
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame) {
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(f.area());

        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " sessionx ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" agent mode", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("  ·  {} agents", self.agents.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        f.render_widget(header, rows[0]);

        let cols = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(rows[1]);
        self.draw_list(f, cols[0]);
        self.draw_preview(f, cols[1]);

        let footer = match self.mode {
            Mode::ConfirmKill => {
                Paragraph::new("kill this agent's pane? y to confirm · any other key cancels")
                    .style(Style::default().fg(Color::Red))
                    .block(Block::default().borders(Borders::TOP))
            }
            Mode::Browse => Paragraph::new("j/k move · h/l scroll · enter jump · x kill · q quit")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::TOP)),
        };
        f.render_widget(footer, rows[2]);
    }

    fn draw_list(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = if self.agents.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  no agents running",
                Style::default().fg(Color::DarkGray),
            )))]
        } else {
            let mut items = vec![];
            let mut last_section: Option<&'static str> = None;
            for a in &self.agents {
                let section = section_title(a.state);
                if last_section != Some(section) {
                    items.push((
                        false,
                        ListItem::new(Line::from(Span::styled(
                            format!(" {section}"),
                            Style::default()
                                .fg(section_color(a.state))
                                .add_modifier(Modifier::BOLD),
                        ))),
                    ));
                    last_section = Some(section);
                }
                items.push((
                    true,
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("  {} ", a.state.dot()),
                            Style::default().fg(state_color(a.state)),
                        ),
                        Span::styled(a.label.clone(), Style::default().fg(state_color(a.state))),
                    ])),
                ));
            }
            items.into_iter().map(|(_, it)| it).collect()
        };

        let mut ls = ListState::default();
        if !self.agents.is_empty() {
            // Map agent index → list index (account for section header rows).
            let mut li = 0;
            let mut last: Option<&'static str> = None;
            for (i, a) in self.agents.iter().enumerate() {
                let section = section_title(a.state);
                if last != Some(section) {
                    li += 1;
                    last = Some(section);
                }
                if i == self.sel {
                    break;
                }
                li += 1;
            }
            ls.select(Some(li));
        }
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" agents "))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD));
        f.render_stateful_widget(list, area, &mut ls);
    }

    fn draw_preview(&mut self, f: &mut Frame, area: Rect) {
        let title = match self.selected() {
            Some(a) => Line::from(vec![
                Span::raw(" "),
                Span::styled(a.state.dot(), Style::default().fg(state_color(a.state))),
                Span::raw(format!(" {} ", a.label)),
            ]),
            None => Line::from(" preview "),
        };
        // Parse the agent's ANSI colors into styled spans, then show the tail
        // end (most recent output). Fall back to plain text if parsing fails.
        let mut text = self
            .preview
            .into_text()
            .unwrap_or_else(|_| Text::raw(self.preview.clone()));
        let inner_w = area.width.saturating_sub(2) as usize;

        // Clamp horizontal scroll to the widest line beyond the viewport.
        let max_w = self.preview_max_width();
        let max_scroll = max_w.saturating_sub(inner_w) as u16;
        self.preview_hscroll = self.preview_hscroll.min(max_scroll);

        // Reserve the bottom inner row for a scrollbar when content overflows.
        let show_bar = max_scroll > 0 && inner_w > 0;
        let text_height = (area.height.saturating_sub(2) as usize).saturating_sub(show_bar as usize);
        if text.lines.len() > text_height {
            text.lines.drain(0..text.lines.len() - text_height);
        }

        // No wrapping: agents render wider than this panel and pad diff/banner
        // lines with a background color out to full width. Wrapping would fold
        // that padding into blank colored bars on the next row; truncating at
        // the panel edge (with horizontal scroll) keeps the backgrounds clean.
        let preview = Paragraph::new(text)
            .scroll((0, self.preview_hscroll))
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(preview, area);

        // A thin scrollbar on the reserved inner row (the border stays intact).
        if show_bar {
            let track = inner_w as u16;
            // Thumb length proportional to visible/total, min 1.
            let total = max_w as u16;
            let thumb = ((track as u32 * track as u32) / total as u32).max(1) as u16;
            let thumb = thumb.min(track);
            // Thumb start proportional to scroll position within the track.
            let span = track - thumb;
            let pos = if max_scroll > 0 {
                (span as u32 * self.preview_hscroll as u32 / max_scroll as u32) as u16
            } else {
                0
            };

            // Track on either side (dim), bright thumb in the middle.
            let thumb_end = (pos + thumb).min(track);
            let line = Line::from(vec![
                Span::styled(
                    "─".repeat(pos as usize),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    "━".repeat((thumb_end - pos) as usize),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    "─".repeat((track - thumb_end) as usize),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            let bar_area = Rect {
                x: area.x + 1,
                y: area.y + area.height - 2,
                width: track,
                height: 1,
            };
            f.render_widget(Paragraph::new(line), bar_area);
        }
    }
}

/// Sort key: lower is more urgent.
fn urgency(s: AgentState) -> u8 {
    match s {
        AgentState::Blocked => 0,
        AgentState::Working => 1,
        AgentState::Done => 2,
        AgentState::Unknown => 3,
        AgentState::Idle => 4,
    }
}

fn section_title(s: AgentState) -> &'static str {
    match s {
        AgentState::Blocked => "needs you",
        AgentState::Working => "running",
        AgentState::Done => "done",
        AgentState::Idle => "idle",
        AgentState::Unknown => "unknown",
    }
}

fn section_color(s: AgentState) -> Color {
    match s {
        AgentState::Blocked => Color::Red,
        _ => Color::DarkGray,
    }
}

fn state_color(s: AgentState) -> Color {
    match s {
        AgentState::Blocked => Color::Red,
        AgentState::Working => Color::Yellow,
        AgentState::Done => Color::Blue,
        AgentState::Idle => Color::Green,
        AgentState::Unknown => Color::DarkGray,
    }
}

pub fn run() -> Result<()> {
    let mut terminal = ratatui::init();
    let res = run_app(&mut terminal);
    ratatui::restore();
    res
}

fn run_app(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    app.reload()?;
    loop {
        terminal.draw(|f| app.draw(f))?;

        if event::poll(Duration::from_millis(1200))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                match app.mode {
                    Mode::ConfirmKill => {
                        if let KeyCode::Char('y') = k.code {
                            app.kill_selected()?;
                        }
                        app.mode = Mode::Browse;
                    }
                    Mode::Browse => match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('j') | KeyCode::Down => app.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.prev(),
                        KeyCode::Char('h') | KeyCode::Left => app.scroll_left(),
                        KeyCode::Char('l') | KeyCode::Right => app.scroll_right(),
                        KeyCode::Enter if app.jump_selected()? => {
                            // Returned from an outside-tmux attach; the
                            // terminal was torn down — re-init it.
                            *terminal = ratatui::init();
                        }
                        KeyCode::Char('x') if app.selected().is_some() => {
                            app.mode = Mode::ConfirmKill;
                        }
                        _ => {}
                    },
                }
            }
        }

        app.reload()?;
    }
    Ok(())
}
