# sessionx shell helpers for fish — source from ~/.config/fish/config.fish
# Provides sxl (list), sxa (attach), sxk (kill), with fzf picker fallback.

function _sessionx_pick_session
    sessionx ls --all 2>/dev/null \
        | fzf --height=40% --reverse --with-nth=1,3 --delimiter=\t \
              --prompt="$argv[1]> " \
        | awk -F\t '{print $1}'
end

function sx
    sessionx $argv
end

function sxl
    sessionx ls --all $argv
end

function sxa
    set -l session
    if test (count $argv) -gt 0
        set session $argv[1]
    else
        set session (_sessionx_pick_session attach)
    end
    test -n "$session"; and sessionx open $session
end

function sxk
    if test (count $argv) -eq 0
        set -l session (_sessionx_pick_session kill)
        test -n "$session"; and sessionx rm $session
    else
        sessionx rm $argv
    end
end

complete -c sxa -f -a "(sessionx ls --all --names-only 2>/dev/null)"
complete -c sxk -f -a "(sessionx ls --all --names-only 2>/dev/null)"
