# PostureFlow bash completion
_postureflow_completions() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="--home --work --dev -t --travel -s --secure -i --status --test -r --reset --default -v --version -h --help"

    if [[ ${cur} == -* ]] ; then
        COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
        return 0
    fi
}
complete -F _postureflow_completions postureflow
complete -F _postureflow_completions pop-profile
