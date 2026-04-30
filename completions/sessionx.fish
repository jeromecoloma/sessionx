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

function __sessionx_themes
    sessionx themes 2>/dev/null
end

complete -c sessionx -n __sessionx_no_subcmd -a init        -d 'scaffold .sessionx.yaml'
complete -c sessionx -n __sessionx_no_subcmd -a edit        -d 'open .sessionx.yaml'
complete -c sessionx -n __sessionx_no_subcmd -a add         -d 'create + attach a session'
complete -c sessionx -n __sessionx_no_subcmd -a ls          -d 'list sessions'
complete -c sessionx -n __sessionx_no_subcmd -a open        -d 'attach to any managed session'
complete -c sessionx -n __sessionx_no_subcmd -a rm          -d 'tear down a session'
complete -c sessionx -n __sessionx_no_subcmd -a completions -d 'print completion script'
complete -c sessionx -n __sessionx_no_subcmd -a themes      -d 'list built-in themes'
complete -c sessionx -n __sessionx_no_subcmd -a theme       -d 'manage project theme (set/preview/list)'
complete -c sessionx -n __sessionx_no_subcmd -a hooks       -d 'manage hook recipes (list/info/install/update/repo)'
complete -c sessionx -n __sessionx_no_subcmd -a config      -d 'manage global config (~/.config/sessionx/config.yaml)'

complete -c sessionx -s v -l verbose -d 'print tmux/git commands'

complete -c sessionx -n '__sessionx_using_cmd rm' -a '(__sessionx_handles)' -d session
complete -c sessionx -n '__sessionx_using_cmd rm' -l force -d 'force removal'

complete -c sessionx -n '__sessionx_using_cmd open' -a '(__sessionx_managed)' -d 'managed session'

complete -c sessionx -n '__sessionx_using_cmd add' -l base -d 'base ref' -x
complete -c sessionx -n '__sessionx_using_cmd add' -l no-attach -d 'do not attach'

complete -c sessionx -n '__sessionx_using_cmd ls' -l names-only -d 'print handles only'
complete -c sessionx -n '__sessionx_using_cmd ls' -l all        -d 'list all managed sessions'

complete -c sessionx -n '__sessionx_using_cmd completions' -a 'bash zsh fish'

# theme subcommand
function __sessionx_theme_action
    set -l cmd (commandline -opc)
    set -l seen_theme 0
    for c in $cmd
        if test $seen_theme -eq 1
            if test "$c" = "$argv[1]"
                return 0
            end
            return 1
        end
        if test "$c" = theme
            set seen_theme 1
        end
    end
    return 1
end

function __sessionx_theme_no_action
    set -l cmd (commandline -opc)
    set -l seen_theme 0
    for c in $cmd
        if test $seen_theme -eq 1
            switch $c
                case set preview
                    return 1
                case '-*'
                case '*'
                    return 1
            end
        end
        if test "$c" = theme
            set seen_theme 1
        end
    end
    test $seen_theme -eq 1
end

complete -c sessionx -n __sessionx_theme_no_action -a 'set'     -d 'set + apply theme'
complete -c sessionx -n __sessionx_theme_no_action -a 'preview' -d 'try a theme without saving'
complete -c sessionx -n __sessionx_theme_no_action -a '(__sessionx_themes)' -d theme
complete -c sessionx -n '__sessionx_theme_action set'     -a '(__sessionx_themes)'
complete -c sessionx -n '__sessionx_theme_action set'     -l no-apply -d 'write yaml only'
complete -c sessionx -n '__sessionx_theme_action set'     -l session  -d 'target session' -x
complete -c sessionx -n '__sessionx_theme_action preview' -a '(__sessionx_themes)'
complete -c sessionx -n '__sessionx_theme_action preview' -l session  -d 'target session' -x

# hooks subcommand
function __sessionx_recipes
    sessionx hooks list 2>/dev/null | awk '/^  [a-z]/{print $1}'
end

function __sessionx_hooks_no_action
    set -l cmd (commandline -opc)
    set -l seen 0
    for c in $cmd
        if test $seen -eq 1
            switch $c
                case list info install update repo
                    return 1
                case '-*'
                case '*'
                    return 1
            end
        end
        if test "$c" = hooks
            set seen 1
        end
    end
    test $seen -eq 1
end

function __sessionx_hooks_action
    set -l cmd (commandline -opc)
    set -l seen 0
    for c in $cmd
        if test $seen -eq 1
            if test "$c" = "$argv[1]"
                return 0
            end
            return 1
        end
        if test "$c" = hooks
            set seen 1
        end
    end
    return 1
end

complete -c sessionx -n __sessionx_hooks_no_action -a list    -d 'show available recipes'
complete -c sessionx -n __sessionx_hooks_no_action -a info    -d 'describe a recipe'
complete -c sessionx -n __sessionx_hooks_no_action -a install -d 'install a recipe'
complete -c sessionx -n __sessionx_hooks_no_action -a update  -d 'refresh the cache'
complete -c sessionx -n __sessionx_hooks_no_action -a repo    -d 'print repo URL/ref/cache path'
complete -c sessionx -n '__sessionx_hooks_action info'    -a '(__sessionx_recipes)'
complete -c sessionx -n '__sessionx_hooks_action install' -a '(__sessionx_recipes)'

# config subcommand
function __sessionx_config_no_action
    set -l cmd (commandline -opc)
    set -l seen 0
    for c in $cmd
        if test $seen -eq 1
            switch $c
                case path get set
                    return 1
                case '-*'
                case '*'
                    return 1
            end
        end
        if test "$c" = config
            set seen 1
        end
    end
    test $seen -eq 1
end

function __sessionx_config_action
    set -l cmd (commandline -opc)
    set -l seen 0
    for c in $cmd
        if test $seen -eq 1
            if test "$c" = "$argv[1]"
                return 0
            end
            return 1
        end
        if test "$c" = config
            set seen 1
        end
    end
    return 1
end

complete -c sessionx -n __sessionx_config_no_action -a path -d 'print config file path'
complete -c sessionx -n __sessionx_config_no_action -a get  -d 'dump config or read a key'
complete -c sessionx -n __sessionx_config_no_action -a set  -d 'write a key=value'
complete -c sessionx -n '__sessionx_config_action get' -a 'agent'
complete -c sessionx -n '__sessionx_config_action set' -a 'agent'
