# Slack Utils justfile

# Default recipe to list available targets
default:
    @just --list

# Launch the interactive TUI
ui:
    cargo run -- ui

# Export conversations from the last 7 days
export-conversations:
    #!/usr/bin/env bash
    from=$(date -d '7 days ago' +%Y-%m-%d)
    to=$(date +%Y-%m-%d)
    cargo run -- export-conversations --from "$from" --to "$to"

# Export conversations with custom date range
export-conversations-range from to:
    cargo run -- export-conversations --from {{from}} --to {{to}}

# Export users
export-users:
    cargo run -- export-users

# Export channels
export-channels:
    cargo run -- export-channels
