#!/usr/bin/env bash
# Shared hook bootstrap for q_terminal.
#
# Hooks spawned by GUI git clients (GitKraken, VS Code, JetBrains) inherit a
# minimal PATH that omits per-user install directories (~/.cargo/bin, ~/.local/bin).
ensure_toolchain_on_path() {
    local candidate
    for candidate in "$HOME/.cargo/bin" "$HOME/.local/bin" /usr/local/bin /opt/homebrew/bin; do
        if [[ -d "$candidate" && ":${PATH}:" != *":${candidate}:"* ]]; then
            export PATH="${candidate}:${PATH}"
        fi
    done

    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: 'cargo' was not found on PATH and is required by this hook." >&2
        echo "       Looked in: ~/.cargo/bin ~/.local/bin /usr/local/bin /opt/homebrew/bin" >&2
        return 1
    fi
}
