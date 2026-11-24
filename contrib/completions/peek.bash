# bash completion for peek
# install: cp peek.bash /usr/share/bash-completion/completions/peek

_peek_paths() {
    local cur=${COMP_WORDS[COMP_CWORD]}
    COMPREPLY=($(compgen -f -- "$cur"))
}

_peek() {
    local cur prev opts
    cur=${COMP_WORDS[COMP_CWORD]}
    prev=${COMP_WORDS[COMP_CWORD-1]}
    opts="--root --limit --height --depth --print --no-preview --interactive --help --version"

    case $prev in
        --root)
            COMPREPLY=($(compgen -d -- "$cur"))
            return
            ;;
        --limit|--height|--depth)
            COMPREPLY=($(compgen -W "10 20 50 100" -- "$cur"))
            return
            ;;
    esac

    if [[ $cur == -* ]]; then
        COMPREPLY=($(compgen -W "$opts" -- "$cur"))
        return
    fi

    _peek_paths
}

complete -o filenames -F _peek peek
