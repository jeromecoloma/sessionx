# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While `sessionx` is pre-1.0, breaking changes may land in minor releases. They
will be called out under a **Breaking** subheading.

## [Unreleased]

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

[Unreleased]: https://github.com/jeromecoloma/sessionx/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.1
[0.1.0]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.0
