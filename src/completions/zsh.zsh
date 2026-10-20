_fpc_interactive() {
    local -a response
    response=("${(@0)$(_FPC_COMPLETE=1 _CLAP_COMPLETE_INDEX=$((CURRENT - 1)) command fpc -- "${words[@]}")}")
    if [[ $response[1] == selected ]]; then
        local selected=$response[2]
        if [[ $selected == */ ]]; then
            compadd -U -S '' -- "$selected"
        else
            compadd -U -- "$selected"
        fi
    elif [[ $response[1] == fallback ]]; then
        _clap_dynamic_completer_fpc "$@"
    fi
}
compdef _fpc_interactive fpc
