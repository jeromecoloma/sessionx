//! `sessionx dash` — the agent-mode navigator (the sidebar TUI).
//!
//! Runs in the left pane of the `sessionx-agentmode` control window (see
//! [`crate::cmd::mode`]). Lists every agent in the session with a live status
//! glyph, lets you focus one onto the stage pane to its right (`swap-pane`),
//! spawn new agents, and kill them. Refreshes state on a timer so the sidebar
//! reflects running/blocked/done as it changes.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::agent;
use crate::agent_state::{self, AgentState, CONTROL_WINDOW, SESSION};
use crate::notify;
use crate::tmux;

struct AgentView {
    id: String,
    handle: String,
    state: AgentState,
    staged: bool,
}

enum Mode {
    Browse,
    NewAgent(String),
}

struct App {
    agents: Vec<AgentView>,
    sel: usize,
    mode: Mode,
    sidebar_pane: String,
    poll_lines: u32,
    /// Last resolved state per pane id, for transition detection.
    prev_states: HashMap<String, AgentState>,
    /// False until the first reload — suppresses a notification burst for
    /// agents that were already blocked/done when the dash started.
    primed: bool,
}

impl App {
    fn new() -> Self {
        App {
            agents: vec![],
            sel: 0,
            mode: Mode::Browse,
            sidebar_pane: std::env::var("TMUX_PANE").unwrap_or_default(),
            poll_lines: 8,
            prev_states: HashMap::new(),
            primed: false,
        }
    }

    /// Re-read the session's panes and recompute each agent's state.
    fn reload(&mut self) -> Result<()> {
        let panes = tmux::list_session_panes(SESSION)?;
        let mut views: Vec<AgentView> = panes
            .iter()
            .filter(|p| p.is_agent)
            .map(|p| {
                let tail = tmux::capture_pane_tail(&p.id, self.poll_lines).unwrap_or_default();
                let state = agent_state::resolve(&p.state_raw, p.seen, &p.current_cmd, &tail);
                let handle = if p.handle.is_empty() {
                    p.id.clone()
                } else {
                    p.handle.clone()
                };

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
                    notify::agent_event(SESSION, &handle, state);
                }

                AgentView {
                    id: p.id.clone(),
                    handle,
                    state,
                    staged: p.window_name == CONTROL_WINDOW,
                }
            })
            .collect();
        views.sort_by(|a, b| a.handle.cmp(&b.handle));
        self.prev_states = views.iter().map(|v| (v.id.clone(), v.state)).collect();
        self.primed = true;
        self.agents = views;
        if self.sel >= self.agents.len() {
            self.sel = self.agents.len().saturating_sub(1);
        }
        Ok(())
    }

    fn next(&mut self) {
        if !self.agents.is_empty() {
            self.sel = (self.sel + 1) % self.agents.len();
        }
    }

    fn prev(&mut self) {
        if !self.agents.is_empty() {
            self.sel = (self.sel + self.agents.len() - 1) % self.agents.len();
        }
    }

    /// Find the pane currently occupying the stage slot (control window, not the
    /// sidebar), if any.
    fn stage_occupant(panes: &[tmux::PaneInfo]) -> Option<String> {
        panes
            .iter()
            .find(|p| p.window_name == CONTROL_WINDOW && !p.is_sidebar)
            .map(|p| p.id.clone())
    }

    /// Swap `target` into the stage slot. When `focus`, leave the cursor on the
    /// staged agent (so the user can type); otherwise return focus to the sidebar.
    fn stage_pane(&self, target: &str, focus: bool) -> Result<()> {
        let panes = tmux::list_session_panes(SESSION)?;
        if let Some(occ) = Self::stage_occupant(&panes) {
            if occ != target {
                tmux::swap_pane(target, &occ)?;
            }
        }
        tmux::set_pane_option(target, "sx-agent-seen", "1")?;
        if focus {
            tmux::select_pane(target)?;
        } else if !self.sidebar_pane.is_empty() {
            tmux::select_pane(&self.sidebar_pane)?;
        }
        Ok(())
    }

    fn stage_selected(&mut self, focus: bool) -> Result<()> {
        let Some(a) = self.agents.get(self.sel) else {
            return Ok(());
        };
        let id = a.id.clone();
        self.stage_pane(&id, focus)
    }

    fn kill_selected(&mut self) -> Result<()> {
        let Some(a) = self.agents.get(self.sel) else {
            return Ok(());
        };
        let target = a.id.clone();
        let panes = tmux::list_session_panes(SESSION)?;
        let staged = panes
            .iter()
            .any(|p| p.id == target && p.window_name == CONTROL_WINDOW);
        if staged {
            // Don't leave the stage empty: park a non-agent pane there first.
            if let Some(park) = panes
                .iter()
                .find(|p| !p.is_agent && !p.is_sidebar)
                .map(|p| p.id.clone())
            {
                tmux::swap_pane(&park, &target)?;
            }
        }
        tmux::kill_pane(&target)?;
        if !self.sidebar_pane.is_empty() {
            tmux::select_pane(&self.sidebar_pane)?;
        }
        Ok(())
    }

    fn create_agent(&mut self, handle: &str) -> Result<()> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let win_name = format!("agent:{handle}");
        let (_w, pid) = tmux::new_window_detached(SESSION, Some(&win_name), &cwd)?;
        tmux::set_pane_option(&pid, "sx-agent", "1")?;
        tmux::set_pane_option(&pid, "sx-agent-handle", handle)?;
        tmux::set_pane_option(&pid, "sx-agent-state", AgentState::Working.as_str())?;
        tmux::set_pane_option(&pid, "sx-agent-seen", "0")?;
        tmux::send_keys(&pid, &agent::resolve())?;
        // Bring the new agent onto the stage at full size and focus it.
        self.stage_pane(&pid, true)?;
        Ok(())
    }

    fn draw(&self, f: &mut Frame) {
        let chunks = Layout::vertical([
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
        ]));
        f.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = if self.agents.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  no agents — press n",
                Style::default().fg(Color::DarkGray),
            )))]
        } else {
            self.agents
                .iter()
                .map(|a| {
                    let mark = if a.staged { "▎" } else { " " };
                    ListItem::new(Line::from(vec![
                        Span::styled(mark, Style::default().fg(Color::Cyan)),
                        Span::raw(format!(" {} ", a.state.glyph())),
                        Span::styled(a.handle.clone(), Style::default().fg(state_color(a.state))),
                    ]))
                })
                .collect()
        };

        let mut ls = ListState::default();
        if !self.agents.is_empty() {
            ls.select(Some(self.sel));
        }
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" agents "))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD));
        f.render_stateful_widget(list, chunks[1], &mut ls);

        let footer = match &self.mode {
            Mode::NewAgent(buf) => Paragraph::new(format!("new agent: {buf}_"))
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::TOP)),
            Mode::Browse => {
                Paragraph::new("j/k move · enter focus · space stage · n new · x kill · q quit")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::TOP))
            }
        };
        f.render_widget(footer, chunks[2]);
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
                let editing = matches!(app.mode, Mode::NewAgent(_));
                if editing {
                    match k.code {
                        KeyCode::Esc => app.mode = Mode::Browse,
                        KeyCode::Enter => {
                            let handle = if let Mode::NewAgent(b) = &app.mode {
                                b.trim().to_string()
                            } else {
                                String::new()
                            };
                            app.mode = Mode::Browse;
                            if !handle.is_empty() {
                                app.create_agent(&handle)?;
                            }
                        }
                        KeyCode::Backspace => {
                            if let Mode::NewAgent(b) = &mut app.mode {
                                b.pop();
                            }
                        }
                        KeyCode::Char(c) => {
                            if let Mode::NewAgent(b) = &mut app.mode {
                                b.push(c);
                            }
                        }
                        _ => {}
                    }
                } else {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('j') | KeyCode::Down => app.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.prev(),
                        KeyCode::Enter | KeyCode::Char('l') => app.stage_selected(true)?,
                        KeyCode::Char(' ') => app.stage_selected(false)?,
                        KeyCode::Char('n') => app.mode = Mode::NewAgent(String::new()),
                        KeyCode::Char('x') => app.kill_selected()?,
                        KeyCode::Char('r') => {}
                        _ => {}
                    }
                }
            }
        }

        app.reload()?;
    }
    Ok(())
}
