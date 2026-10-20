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

function _fpc_native_tab
    set -l words (commandline --current-process --tokens-expanded --cut-at-cursor)
    set -l current (commandline --current-token --tokens-expanded | string collect)
    if test "$words[1]" = fpc; and test -n "$current"; and not commandline --paging-mode
        set -l candidates (COMPLETE=fish command fpc -- $words "$current")
        if set -q candidates[1]
            set -l first (string split --max 1 \t -- "$candidates[1]")[1]
            # Fish's subsequence filter is case-sensitive. Use the top candidate's
            # casing for the same query, then let the existing Tab binding complete it.
            set -l pattern ''
            for character in (string split '' -- "$current")
                set pattern "$pattern(?>.*?("(string escape --style=regex -- "$character")"))"
            end
            set -l matched (string match --ignore-case --regex --groups-only -- "$pattern" "$first")
            if test (count $matched) -eq (string length -- "$current")
                set -l canonical (string join '' -- $matched)
                if test "$canonical" != "$current"
                    commandline --current-token --replace -- (string escape -- "$canonical")
                end
            end
        end
    end
    _fpc_previous_tab
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
        if not contains -- _fpc_native_tab $actions
            set -g _fpc_tab_$mode $actions
        end
    else
        set -g _fpc_tab_$mode complete
    end
    bind -M $mode \t _fpc_native_tab
end
