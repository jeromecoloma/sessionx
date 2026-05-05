# sessionx shell helpers — source from your .zshrc / .bashrc
# Provides sxl (list), sxa (attach), sxk (kill), with fzf picker fallback
# and zsh tab-completion of managed session names.

# Make re-sourcing safe: drop any aliases that would shadow our function names.
unalias sx sxl sxa sxk 2>/dev/null

_sessionx_pick_session() {
    sessionx ls --all 2>/dev/null \
        | fzf --height=40% --reverse --with-nth=1,3 --delimiter=$'\t' \
              --prompt="$1> " \
        | awk -F'\t' '{print $1}'
}

sx() {
    sessionx "$@"
}

sxl() {
    sessionx ls --all "$@"
}

sxa() {
    local session
    if (( $# )); then
        session="$1"
    else
        session="$(_sessionx_pick_session attach)"
    fi
    [ -n "$session" ] && sessionx open "$session"
}

sxk() {
    if (( $# == 0 )); then
        local session
        session="$(_sessionx_pick_session kill)"
        [ -n "$session" ] && sessionx rm "$session"
    else
        sessionx rm "$@"
    fi
}

if [ -n "${ZSH_VERSION:-}" ] && (( $+functions[compdef] )); then
    _sessionx_helper_sessions() {
        local -a sessions
        sessions=(${(f)"$(sessionx ls --all --names-only 2>/dev/null)"})
        compadd -a sessions
    }
    compdef _sessionx_helper_sessions sxa sxk
    compdef sx=sessionx
fi
