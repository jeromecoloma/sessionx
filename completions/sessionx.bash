# bash completion for sessionx

_sessionx() {
    local cur prev words cword
    _init_completion || return

    local commands="init edit add ls open rm completions themes -h --help -V --version -v --verbose"
    local cmd=""
    local i
    for ((i=1; i < cword; i++)); do
        case "${words[i]}" in
            -*) ;;
            *) cmd="${words[i]}"; break ;;
        esac
    done

    if [[ -z "$cmd" ]]; then
        COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
        return
    fi

    case "$cmd" in
        rm)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--force" -- "$cur") )
            else
                local handles
                handles="$(sessionx ls --names-only 2>/dev/null)"
                COMPREPLY=( $(compgen -W "$handles" -- "$cur") )
            fi
            ;;
        open)
            local managed
            managed="$(sessionx open --names-only 2>/dev/null)"
            COMPREPLY=( $(compgen -W "$managed" -- "$cur") )
            ;;
        add)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--base --no-attach" -- "$cur") )
            fi
            ;;
        ls)
            COMPREPLY=( $(compgen -W "--names-only --all" -- "$cur") )
            ;;
        completions)
            COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
            ;;
    esac
}

complete -F _sessionx sessionx
