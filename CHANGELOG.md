# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While `sessionx` is pre-1.0, breaking changes may land in minor releases. They
will be called out under a **Breaking** subheading.

## [Unreleased]

### Changed
- **Agent mode rebuilt as an attention inbox** (`sessionx mode agent`).
  Agents now live in ordinary panes in ordinary sessions — the dedicated
  `sessionx-agentmode` session, hidden agent windows, and the sidebar/stage
  `swap-pane` machinery are gone. The dashboard is a full-screen TUI over
  every agent pane on the tmux server, sorted by urgency (needs you /
  running / done / idle) with a live tail preview of the selected agent.
  `Enter` jumps to the agent's real pane (`switch-client` inside tmux,
  attach from outside) and marks it reviewed.

### Added
- `sessionx agent-hooks <install|uninstall|status>` — wires Claude Code hooks
  into `~/.claude/settings.json` so agents report state natively:
  `UserPromptSubmit` → working, `Notification` → blocked, `Stop` → done.
  Idempotent; preserves unrelated hooks; `uninstall` removes exactly what
  was added.
- Hook-less agents (codex, aider, gemini, …) are auto-detected by foreground
  process name and tracked via the existing heuristics.
- Shell completions for `mode` and `agent-hooks` (bash/zsh/fish).

### Breaking
- `sessionx mode agent` no longer creates the `sessionx-agentmode` session,
  and the dashboard no longer spawns agents (`n`) — create sessions the
  normal way (`sessionx add`, `.sessionx.yaml` agent windows). A leftover
  `sessionx-agentmode` session from v0.2.0 is just a plain session now; kill
  it with `tmux kill-session -t sessionx-agentmode`.

## [0.2.0] - 2026-06-12

### Added
- **Agent mode** (`sessionx mode agent`): a `sessionx-agentmode` session with a
  sidebar navigator (ratatui TUI) and a stage pane. Create, focus, and kill
  agents from the sidebar; each agent lives in its own hidden window and is
  swapped onto the stage when focused. Live status glyphs: 🔴 blocked,
  🟡 working, 🔵 done, 🟢 idle.
- `sessionx agent-state <blocked|working|done|idle>` — lets agent integrations
  (e.g. Claude Code hooks) report their status via pane-scoped tmux options.
  Agents without hooks fall back to heuristics (shell-exit detection,
  prompt-scrape of the pane tail).
- **Desktop notifications**: transitions into blocked/done emit an OSC 9
  notification + BEL written directly to every attached tmux client tty
  (bypasses passthrough). Banners on Ghostty/kitty/iTerm2/WezTerm/foot;
  bell-only fallback elsewhere.
- **Tab-title status glyphs**: every agent state transition rewrites the outer
  terminal's title (OSC 2) with the state glyph, so tab bars show at a glance
  which agent needs attention. Plain sessions (no handle) are labeled
  `session:window`.
- Global config keys `notify`, `bell`, `title` — toggle via
  `sessionx config set <key> true|false` (all default on); `SX_NOTIFY=0`
  disables notifications per-shell.

## [0.1.13] - 2026-06-05

### Added
- Plain tmux sessions are now tracked: they're tagged as sessionx-managed so
  they appear in `sxla` (`sessionx ls --all`) with their directory as the
  branch/path, even though they have no `.sessionx.yaml`.

### Changed
- Creating a **New plain tmux session** now always offers a rename prompt
  before the session is created (esc keeps the generated name), matching the
  rename flow used when a session name is too long.

### Fixed
- `sxk` on a plain tmux session no longer prints the spurious
  "project config not found … worktree cleanup skipped" warning — plain
  sessions have no worktree to clean up.

## [0.1.12] - 2026-05-23

### Added
- `sxla` shell helper: `sessionx ls --all` (every managed session globally).
  `sxl` now maps to `sessionx ls` (current project only).

### Changed
- `sessionx ls --all` pretty-prints to a TTY with aligned columns, a dim/bold
  header, bold-cyan session names, yellow branches, dim paths, and `$HOME`
  collapsed to `~`. A blank line separates the current project's sessions
  from the rest. Piped output is unchanged (raw tab-separated) so `fzf` and
  scripts keep working. Respects `NO_COLOR`.
- `sessionx` interactive picker (fzf) now uses `--cycle`, so Ctrl-P/Ctrl-N
  wrap around at the ends of the list.
- When invoked from a directory that is neither a sessionx project nor a git
  repo, the picker now lists **New plain tmux session** as the first entry
  (instead of burying it at the bottom).

## [0.1.11] - 2026-05-12

### Changed
- `sessionx ls --all` now sorts sessions belonging to the current project
  (detected via `.sessionx.yaml` in cwd or any parent) to the top, so
  `sxa`'s fzf picker surfaces related sessions first.

## [0.1.10] - 2026-05-07

### Fixed
- `sessionx rm <name> [--force]` recovers from a partial teardown where the
  tmux session was killed but the worktree removal failed (e.g. untracked
  files without `--force`). Accepts either a handle or the full session name.
- `sessionx` interactive picker now lists leftover worktrees in the current
  project as **Clean up orphan worktree** entries; enter or `ctrl-x` prompts
  to force-remove.

## [0.1.9] - 2026-05-06

### Fixed
- `sessionx shell-init zsh|bash` now bundles the completion script
  inline, so `compdef sx=sessionx` (in the helpers) works for users who
  installed via `cargo install sessionx` and never registered the static
  completion file.

## [0.1.8] - 2026-05-06

### Added
- `sessionx shell-init <bash|zsh|fish>` prints the `sx`/`sxl`/`sxa`/`sxk`
  helpers (and zsh completions) for `eval`. Lets `cargo install sessionx`
  users wire up the shortcuts without cloning the repo:
  `eval "$(sessionx shell-init zsh)"`.

## [0.1.7] - 2026-05-05

### Changed
- Default project template's second window is now a plain `shell`
  (`exec $SHELL`) instead of `${EDITOR:-vi} .`. The same applies to the
  PHP/Laravel auto-detected layout.

### Fixed
- Trimmed a stray trailing space in the inactive-window status format,
  and removed the placeholder space in the last-window indicator's
  off-branch so non-last windows no longer carry phantom padding.

## [0.1.6] - 2026-05-05

### Fixed
- `sessionx rm <name>` (and the `sxk` shell helper) now works on
  renamed managed sessions. The argument is resolved against the
  managed-session registry first, so a session whose name no longer
  matches `prefix+handle` is killed and its worktree removed in the
  correct project.

## [0.1.5] - 2026-05-05

### Fixed
- `sessionx add` rename prompt no longer silently falls back to the
  long auto-generated name when the chosen rename collides with an
  existing tmux session or sanitizes to empty. The prompt now reports
  the conflict and re-asks; Esc still keeps the original name.

## [0.1.4] - 2026-05-05

### Added
- `sx`, `sxl`, `sxa`, `sxk` shell helpers (bash/zsh/fish) shipped in
  `shell/sessionx-helpers.{sh,fish}`. `sx` is a passthrough shortcut
  for `sessionx`; `sxl` lists every managed session globally; `sxa`
  attaches and `sxk` removes, both with an `fzf` picker fallback when
  no name is given. zsh tab-completion of managed session names is
  registered for `sxa`/`sxk`, and `sx` inherits `sessionx`'s
  completion. `install.sh` offers to source the helpers from your
  shell rc.

## [0.1.3] - 2026-05-03

### Added
- Theme picker entry in the no-arg picker for plain (unmanaged) tmux
  sessions, so themes can be applied to any tmux session.
- Long-session-name guard: when an auto-generated session name exceeds
  20 characters, `sessionx add` and the picker prompt for a shorter
  name before creating the session.
- Nested-session detection: the picker and `sessionx add` now refuse
  to run inside a sessionx-attached tmux session. Set
  `SESSIONX_ALLOW_NESTED=1` to override. Detection uses the
  `@sessionx-managed` user option plus a `SESSIONX_ACTIVE` tmux env
  var exported on attach/switch.

### Fixed
- `sessionx ls` now finds sessions that were renamed at creation time
  by matching managed sessions against the current project root, not
  just the prefix.

## [0.1.2] - 2026-05-02

### Added
- `ctrl-x` keybinding in the no-arg picker to delete a managed session
  (with confirmation). Runs full cleanup (pre_remove hooks + worktree
  removal) when the project's `.sessionx.yaml` is reachable, otherwise
  falls back to killing just the tmux session. Requires `fzf`.

## [0.1.1] - 2026-05-01

### Added
- Interactive `sessionx init` wizard with detected-stack defaults.
- Hooks fetcher and `sessionx config` command.
- No-arg interactive session picker (fzf when available, `inquire` fallback).
- New `status.window_id_style` and `status.pane_id_style` options for per-session
  glyph styles. Allowed values: `fsquare`, `hsquare`, `dsquare`, `super`, `sub`,
  `roman`, `digital`, `none`, `hide`. Validated at apply time.
- Optional git-status segment (branch + dirty/clean dot) on the right side
  for every theme except `minimal`.
- Cargo-release config for one-command version bumps.
- Pre-push git hook documentation.
- Open source community files: `LICENSE`, `CODE_OF_CONDUCT.md`, `GOVERNANCE.md`,
  `MAINTAINERS.md`, `CONTRIBUTING.md`, `SECURITY.md`, issue/PR templates,
  CI and release workflows.

### Changed
- All bundled themes now render a richer per-session status bar inspired by
  `tokyo-night-tmux`: OS-icon block, prefix-aware session indicator with a
  dedicated `session_bg` (so the session label no longer collides with the
  active-window pill), shell-vs-ssh window icon, NerdFont boxed window numbers
  (`fsquare` default), superscript pane count (`super` default), last-window
  arrow (`󰁯`).
- Active window now renders as a raised pill on a `dim` background with
  powerline triangle separators and a curated `muted` mid-tone for inactive
  window names — sharper focus contrast.

## [0.1.0] - 2026-04-29

Initial release.

### Added
- `sessionx init`, `add`, `ls`, `rm`, `edit`, `open` commands.
- Per-project tmux status bars scoped to spawned sessions.
- Built-in themes: `tokyo-night`, `catppuccin`, `dracula`, `gruvbox`,
  `nord`, `rose-pine`, `minimal`.
- `sessionx themes`, `sessionx theme`, `sessionx theme set`,
  `sessionx theme preview`.
- Optional git-worktree mode via `worktree_dir:` in `.sessionx.yaml`.
- `post_create` and `pre_remove` hooks with `SX_*` env vars.
- Shell completions for `bash`, `zsh`, `fish`.

[Unreleased]: https://github.com/jeromecoloma/sessionx/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.2.0
[0.1.13]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.13
[0.1.12]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.12
[0.1.11]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.11
[0.1.10]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.10
[0.1.9]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.9
[0.1.8]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.8
[0.1.7]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.7
[0.1.6]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.6
[0.1.5]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.5
[0.1.4]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.4
[0.1.3]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.3
[0.1.2]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.2
[0.1.1]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.1
[0.1.0]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.0
