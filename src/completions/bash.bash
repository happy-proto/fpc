_fpc_interactive() {
    local result_kind selected
    COMPREPLY=()
    {
        IFS= read -r -d '' result_kind
        if [[ $result_kind == selected ]]; then
            IFS= read -r -d '' selected
            COMPREPLY=("$selected")
            compopt -o filenames
            if [[ $selected == */ ]]; then
                compopt -o nospace
            else
                compopt +o nospace
            fi
        fi
    } < <(_FPC_COMPLETE=1 _CLAP_COMPLETE_INDEX="$COMP_CWORD" command fpc -- "${COMP_WORDS[@]}")
    if [[ $result_kind == fallback ]]; then
        _clap_complete_fpc "$@"
    fi
}
complete -o filenames -o nospace -F _fpc_interactive fpc
