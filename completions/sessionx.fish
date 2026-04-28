# fish completion for sessionx

function __sessionx_no_subcmd
    set -l cmd (commandline -opc)
    set -e cmd[1]
    for c in $cmd
        switch $c
            case -v --verbose -h --help -V --version
            case '*'
                return 1
        end
    end
    return 0
end

function __sessionx_using_cmd
    set -l cmd (commandline -opc)
    set -e cmd[1]
    for c in $cmd
        switch $c
            case -v --verbose
            case $argv[1]
                return 0
            case '*'
                return 1
        end
    end
    return 1
end

function __sessionx_handles
    sessionx ls --names-only 2>/dev/null
end

function __sessionx_managed
    sessionx open --names-only 2>/dev/null
end

complete -c sessionx -n __sessionx_no_subcmd -a init        -d 'scaffold .sessionx.yaml'
complete -c sessionx -n __sessionx_no_subcmd -a edit        -d 'open .sessionx.yaml'
complete -c sessionx -n __sessionx_no_subcmd -a add         -d 'create + attach a session'
complete -c sessionx -n __sessionx_no_subcmd -a ls          -d 'list sessions'
complete -c sessionx -n __sessionx_no_subcmd -a open        -d 'attach to any managed session'
complete -c sessionx -n __sessionx_no_subcmd -a rm          -d 'tear down a session'
complete -c sessionx -n __sessionx_no_subcmd -a completions -d 'print completion script'

complete -c sessionx -s v -l verbose -d 'print tmux/git commands'

complete -c sessionx -n '__sessionx_using_cmd rm' -a '(__sessionx_handles)' -d session
complete -c sessionx -n '__sessionx_using_cmd rm' -l force -d 'force removal'

complete -c sessionx -n '__sessionx_using_cmd open' -a '(__sessionx_managed)' -d 'managed session'

complete -c sessionx -n '__sessionx_using_cmd add' -l base -d 'base ref' -x
complete -c sessionx -n '__sessionx_using_cmd add' -l no-attach -d 'do not attach'

complete -c sessionx -n '__sessionx_using_cmd ls' -l names-only -d 'print handles only'
complete -c sessionx -n '__sessionx_using_cmd ls' -l all        -d 'list all managed sessions'

complete -c sessionx -n '__sessionx_using_cmd completions' -a 'bash zsh fish'
