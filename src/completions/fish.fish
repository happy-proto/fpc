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

function _fpc_interactive
    set -l words (commandline --current-process --tokens-expanded --cut-at-cursor)
    if not contains -- "$words[1]" fpc
        _fpc_previous_tab
        return
    end
    set -l current (commandline --current-token --tokens-expanded | string collect)
    set -l result (env _FPC_COMPLETE=1 _CLAP_COMPLETE_INDEX=(count $words) fpc -- $words "$current" | string split0)
    switch "$result[1]"
        case fallback
            commandline -f complete
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
    if test -n "$key_index"
        set -l actions $parts[(math $key_index + 1)..-1]
        if not contains -- _fpc_interactive $actions
            set -g _fpc_tab_$mode $actions
        end
    else
        set -g _fpc_tab_$mode complete
    end
    bind -M $mode \t _fpc_interactive
end
