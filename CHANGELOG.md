# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While `sessionx` is pre-1.0, breaking changes may land in minor releases. They
will be called out under a **Breaking** subheading.

## [Unreleased]

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

[Unreleased]: https://github.com/jeromecoloma/sessionx/compare/v0.1.6...HEAD
[0.1.6]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.6
[0.1.5]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.5
[0.1.4]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.4
[0.1.3]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.3
[0.1.2]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.2
[0.1.1]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.1
[0.1.0]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.0
