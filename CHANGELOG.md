# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While `sessionx` is pre-1.0, breaking changes may land in minor releases. They
will be called out under a **Breaking** subheading.

## [Unreleased]

### Added
- Open source community files: `LICENSE`, `CODE_OF_CONDUCT.md`, `GOVERNANCE.md`,
  `MAINTAINERS.md`, `CONTRIBUTING.md`, `SECURITY.md`, issue/PR templates,
  CI and release workflows.

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

[Unreleased]: https://github.com/jeromecoloma/sessionx/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jeromecoloma/sessionx/releases/tag/v0.1.0
