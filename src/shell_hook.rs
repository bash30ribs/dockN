pub struct ShellHook;

impl ShellHook {
    pub fn zsh() -> &'static str {
        r#"# dockN Subconscious Shell Integration for ZSH
# Add this to your ~/.zshrc: eval "$(dockn hook zsh)"

autoload -Uz add-zsh-hook 2>/dev/null

_dockn_preexec() {
    _dockn_cmd_start_time=${EPOCHREALTIME:-$(date +%s.%N 2>/dev/null || date +%s)}
    _dockn_last_command="$1"
}

_dockn_precmd() {
    local exit_code=$?
    if [ -n "$_dockn_cmd_start_time" ] && [ -n "$_dockn_last_command" ]; then
        local end_time=${EPOCHREALTIME:-$(date +%s.%N 2>/dev/null || date +%s)}
        local dur_ms=0
        if which python3 >/dev/null 2>&1; then
            dur_ms=$(python3 -c "print(int(max(0, ($end_time - $_dockn_cmd_start_time) * 1000)))" 2>/dev/null || echo 0)
        fi
        
        # Asynchronously record to digital subconscious without blocking terminal prompt
        (dockn record-cmd \
            --comm "${_dockn_last_command%% *}" \
            --cmdline "$_dockn_last_command" \
            --cwd "$PWD" \
            --duration-ms "$dur_ms" \
            --exit-code "$exit_code" >/dev/null 2>&1 &)
    fi
    unset _dockn_cmd_start_time
    unset _dockn_last_command
}

add-zsh-hook preexec _dockn_preexec
add-zsh-hook precmd _dockn_precmd
"#
    }

    pub fn bash() -> &'static str {
        r#"# dockN Subconscious Shell Integration for BASH
# Add this to your ~/.bashrc: eval "$(dockn hook bash)"

_dockn_bash_preexec() {
    if [ -z "$_dockn_in_prompt" ]; then
        _dockn_cmd_start_time=$(date +%s.%N 2>/dev/null || date +%s)
        _dockn_last_command="$BASH_COMMAND"
    fi
}

_dockn_bash_precmd() {
    local exit_code=$?
    _dockn_in_prompt=1
    if [ -n "$_dockn_cmd_start_time" ] && [ -n "$_dockn_last_command" ]; then
        local end_time=$(date +%s.%N 2>/dev/null || date +%s)
        local dur_ms=0
        if which python3 >/dev/null 2>&1; then
            dur_ms=$(python3 -c "print(int(max(0, ($end_time - $_dockn_cmd_start_time) * 1000)))" 2>/dev/null || echo 0)
        fi

        (dockn record-cmd \
            --comm "${_dockn_last_command%% *}" \
            --cmdline "$_dockn_last_command" \
            --cwd "$PWD" \
            --duration-ms "$dur_ms" \
            --exit-code "$exit_code" >/dev/null 2>&1 &)
    fi
    unset _dockn_cmd_start_time
    unset _dockn_last_command
    _dockn_in_prompt=""
}

trap '_dockn_bash_preexec' DEBUG
PROMPT_COMMAND="_dockn_bash_precmd${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
"#
    }
}
