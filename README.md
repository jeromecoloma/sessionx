# sessionx

A small tmux session manager. Sits between [tmuxp](https://tmuxp.git-pull.com/) and [workmux](https://workmux.raine.dev/) — declarative YAML, optional git-worktree mode, pre/post hooks, and **per-project tmux status bars**.

## Requirements

- **Rust toolchain** (stable, 1.75+) — install via [rustup](https://rustup.rs):
  ```sh
  brew install rustup        # macOS, if you don't have it
  rustup default stable
  ```
  Homebrew installs `rustup` keg-only, so `cargo`/`rustc` aren't on `PATH` by default. Add the shim dir:
  ```sh
  echo 'export PATH="/opt/homebrew/opt/rustup/bin:$PATH"' >> ~/.zshrc
  source ~/.zshrc
  ```
- **tmux** 3.0+ — `brew install tmux`
- **git** 2.5+ (only needed for worktree mode) — `brew install git`
- **bash** (for hook execution) — present on macOS by default

## Install

Easy way:

```sh
./install.sh             # interactive — builds, asks about completions
./install.sh --yes       # non-interactive (uses your $SHELL)
./install.sh --no-completions
./install.sh --completions-only --shell zsh
```

Manual:

```sh
cargo install --path .
export PATH="$HOME/.cargo/bin:$PATH"   # if not already
```

## Quick start

```sh
cd my-project
sessionx init                  # writes .sessionx.yaml
sessionx add work              # creates session "<project>-work" and attaches
sessionx ls
sessionx rm work
```

## Modes

- **plain** (default): `sessionx add <name>` spawns a tmux session in the project directory.
- **worktree**: set `worktree_dir:` in `.sessionx.yaml` and `add` will also create a git worktree + branch named `<name>`. `rm` tears it down.

## Status bar

The headline feature. `status:` in `.sessionx.yaml` is applied **scoped to the spawned session only** — other tmux sessions are untouched.

```yaml
status:
  style:
    status_style: "bg=#1e1e2e,fg=#cdd6f4"
    window_status_current_style: "bg=#89b4fa,fg=#1e1e2e,bold"
  left: " #S "
  right: " #(~/.sessionx/segments/clock.sh) "
  segments:
    - name: clock
      command: "date +%H:%M:%S"
  status_interval: 1
```

Style keys map 1:1 to tmux options — `status_style` becomes `status-style`, etc.

Custom `segments` are materialized into `~/.sessionx/segments/<name>.sh` and refreshed by tmux's built-in `#(...)` polling every `status_interval` seconds. No daemon.

## Hooks

`post_create` runs after the worktree (or in the project root, plain mode) and **before** tmux launches. `pre_remove` runs before teardown. Both receive these env vars:

| Var | Always |
|---|---|
| `SX_PROJECT_ROOT`, `SX_HANDLE`, `SX_SESSION_NAME` | yes |
| `SX_WORKTREE_PATH`, `SX_BRANCH_NAME` | worktree mode |
| `SX_ICON_<NAME>` | from `status.icons` |

These mirror workmux's `WM_*` vars — porting an existing workmux script is mostly `sed s/WM_/SX_/`.

## Commands

| Command | What it does |
|---|---|
| `sessionx init` | Write a starter `.sessionx.yaml`. |
| `sessionx edit` | Open `.sessionx.yaml` in `$VISUAL`/`$EDITOR`. |
| `sessionx add <name> [--base <ref>] [--no-attach]` | Create + attach. |
| `sessionx ls` | List sessions managed by this project. |
| `sessionx rm <name> [--force]` | Tear down. |

Add `-v` to any command to see the underlying `tmux`/`git` calls.

## Shell completions

```sh
# zsh — write to a dir on $fpath
sessionx completions zsh > "${fpath[1]}/_sessionx"

# bash
sessionx completions bash > /usr/local/etc/bash_completion.d/sessionx

# fish
sessionx completions fish > ~/.config/fish/completions/sessionx.fish
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.
