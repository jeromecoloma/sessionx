#!/usr/bin/env bash
# sessionx installer
#
# Usage:
#   ./install.sh                       # interactive: builds, asks about completions
#   ./install.sh --yes                 # non-interactive: install + auto-detect shell completion
#   ./install.sh --no-completions      # skip completion install
#   ./install.sh --shell zsh           # force a shell (zsh|bash|fish)
#   ./install.sh --completions-only    # skip cargo install, only set up completions

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
ASSUME_YES=0
SKIP_COMPLETIONS=0
COMPLETIONS_ONLY=0
FORCE_SHELL=""

while (($#)); do
    case "$1" in
        -y|--yes)               ASSUME_YES=1 ;;
        --no-completions)       SKIP_COMPLETIONS=1 ;;
        --completions-only)     COMPLETIONS_ONLY=1 ;;
        --shell)                FORCE_SHELL="${2:-}"; shift ;;
        -h|--help)
            sed -n '2,12p' "$0"; exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
    shift
done

log()  { printf '\033[1;34m[sessionx]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[sessionx]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[sessionx]\033[0m %s\n' "$*" >&2; exit 1; }

ask_yn() {
    local prompt="$1" default="${2:-y}" reply
    if (( ASSUME_YES )); then return 0; fi
    read -r -p "$prompt [Y/n] " reply
    reply="${reply:-$default}"
    [[ "$reply" =~ ^[Yy] ]]
}

#-------------------------------------------------------------------------------
# 1. Build & install the binary
#-------------------------------------------------------------------------------
if (( ! COMPLETIONS_ONLY )); then
    command -v cargo >/dev/null || die "cargo not found — install rustup first (see README)."
    log "building & installing sessionx via cargo install --path $SCRIPT_DIR --force"
    cargo install --path "$SCRIPT_DIR" --force

    cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
    if [[ ":$PATH:" != *":$cargo_bin:"* ]]; then
        warn "$cargo_bin is not on PATH — add this to your shell rc:"
        warn "  export PATH=\"$cargo_bin:\$PATH\""
    fi
fi

#-------------------------------------------------------------------------------
# 2. Completions
#-------------------------------------------------------------------------------
if (( SKIP_COMPLETIONS )); then
    log "skipping completions (--no-completions)"
    exit 0
fi

# Resolve which sessionx to ask for completions
SX_BIN="$(command -v sessionx || true)"
if [[ -z "$SX_BIN" ]]; then
    SX_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/sessionx"
fi
[[ -x "$SX_BIN" ]] || die "sessionx binary not found at $SX_BIN"

# Detect shell
detect_shell() {
    if [[ -n "$FORCE_SHELL" ]]; then echo "$FORCE_SHELL"; return; fi
    case "$(basename "${SHELL:-}")" in
        zsh)  echo zsh ;;
        bash) echo bash ;;
        fish) echo fish ;;
        *)    echo unknown ;;
    esac
}

SHELL_KIND="$(detect_shell)"
if [[ "$SHELL_KIND" == "unknown" ]]; then
    warn "couldn't detect shell. Re-run with --shell zsh|bash|fish."
    exit 0
fi

if ! ask_yn "Install $SHELL_KIND completions?"; then
    log "skipping completion install"
    exit 0
fi

case "$SHELL_KIND" in
    zsh)
        target_dir="$HOME/.zsh/completions"
        target_file="$target_dir/_sessionx"
        mkdir -p "$target_dir"
        "$SX_BIN" completions zsh > "$target_file"
        log "wrote $target_file"

        zshrc="$HOME/.zshrc"
        marker='# sessionx completions'
        if [[ -f "$zshrc" ]] && grep -qF "$marker" "$zshrc"; then
            log "$zshrc already configured"
        else
            cat >> "$zshrc" <<EOF

$marker
fpath=($target_dir \$fpath)
autoload -Uz compinit && compinit
EOF
            log "appended fpath + compinit to $zshrc"
        fi

        rm -f "$HOME/.zcompdump"*
        log "cleared zsh completion cache — restart your shell or run: exec zsh"
        ;;

    bash)
        if [[ -d /opt/homebrew/etc/bash_completion.d ]]; then
            target_dir="/opt/homebrew/etc/bash_completion.d"
        elif [[ -d /usr/local/etc/bash_completion.d ]]; then
            target_dir="/usr/local/etc/bash_completion.d"
        else
            target_dir="$HOME/.local/share/bash-completion/completions"
            mkdir -p "$target_dir"
        fi
        target_file="$target_dir/sessionx"
        "$SX_BIN" completions bash > "$target_file"
        log "wrote $target_file"
        log "restart your shell or run: source $target_file"
        ;;

    fish)
        target_dir="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
        mkdir -p "$target_dir"
        target_file="$target_dir/sessionx.fish"
        "$SX_BIN" completions fish > "$target_file"
        log "wrote $target_file"
        log "fish auto-loads completions — open a new shell to use them"
        ;;
esac

log "done."
