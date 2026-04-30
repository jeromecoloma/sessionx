# bash completion for sessionx

_sessionx() {
    local cur prev words cword
    _init_completion || return

    local commands="init edit add ls open rm completions themes theme hooks config -h --help -V --version -v --verbose"
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
        hooks)
            local sub=""
            local j
            for ((j=i+1; j < cword; j++)); do
                case "${words[j]}" in
                    -*) ;;
                    *) sub="${words[j]}"; break ;;
                esac
            done
            if [[ -z "$sub" ]]; then
                COMPREPLY=( $(compgen -W "list info install update repo" -- "$cur") )
            else
                case "$sub" in
                    info|install)
                        local recipes
                        recipes="$(sessionx hooks list 2>/dev/null | awk '/^  [a-z]/{print $1}')"
                        COMPREPLY=( $(compgen -W "$recipes" -- "$cur") )
                        ;;
                esac
            fi
            ;;
        config)
            local sub=""
            local j
            for ((j=i+1; j < cword; j++)); do
                case "${words[j]}" in
                    -*) ;;
                    *) sub="${words[j]}"; break ;;
                esac
            done
            if [[ -z "$sub" ]]; then
                COMPREPLY=( $(compgen -W "path get set" -- "$cur") )
            else
                case "$sub" in
                    get|set)
                        COMPREPLY=( $(compgen -W "agent" -- "$cur") )
                        ;;
                esac
            fi
            ;;
        theme)
            local sub=""
            local j
            for ((j=i+1; j < cword; j++)); do
                case "${words[j]}" in
                    -*) ;;
                    *) sub="${words[j]}"; break ;;
                esac
            done
            local themes
            themes="$(sessionx themes 2>/dev/null)"
            if [[ -z "$sub" ]]; then
                if [[ "$cur" == -* ]]; then
                    COMPREPLY=( $(compgen -W "--no-apply --session" -- "$cur") )
                else
                    COMPREPLY=( $(compgen -W "set preview $themes" -- "$cur") )
                fi
            else
                case "$sub" in
                    set|preview)
                        if [[ "$cur" == -* ]]; then
                            COMPREPLY=( $(compgen -W "--no-apply --session" -- "$cur") )
                        else
                            COMPREPLY=( $(compgen -W "$themes" -- "$cur") )
                        fi
                        ;;
                esac
            fi
            ;;
    esac
}

complete -F _sessionx sessionx
