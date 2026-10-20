if not status is-interactive
    return
end

function _fpc_previous_tab
    set -l saved _fpc_tab_$fish_bind_mode
    for action in $$saved
        if contains -- "$action" (bind --function-names)
            commandline -f $action
        else
            eval $action
        end
    end
end

function _fpc_inline_tab
    set -l words (commandline --current-process --tokens-expanded --cut-at-cursor)
    set -l current (commandline --current-token --tokens-expanded | string collect)
    if test "$words[1]" != fpc; or commandline --paging-mode
        _fpc_previous_tab
        return
    end
    set -l result (_FPC_COMPLETE=1 command fpc -- $words "$current" | string split0)
    switch "$result[1]"
        case fallback
            _fpc_previous_tab
        case selected
            commandline --current-token --replace -- (string escape -- "$result[2]")
            commandline -f repaint
        case '*'
            commandline -f repaint
    end
end

for mode in default insert
    if not contains -- $mode (bind --list-modes)
        continue
    end
    set -l binding (bind --user -M $mode \t 2>/dev/null)
    if test (count $binding) -eq 0
        set binding (bind --preset -M $mode \t 2>/dev/null)
    end
    printf '%s\n' $binding | read --tokenize --array parts
    set -l key_index (contains --index -- tab $parts)
    if test -z "$key_index"
        set key_index (contains --index -- \t $parts)
    end
    if test -n "$key_index"
        set -l actions $parts[(math $key_index + 1)..-1]
        if not contains -- _fpc_inline_tab $actions
            set -g _fpc_tab_$mode $actions
        end
    else
        set -g _fpc_tab_$mode complete
    end
    bind -M $mode \t _fpc_inline_tab
end
