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

# Download attachments from conversations
download-attachments:
    cargo run -- download-attachments

# Download attachments with custom paths
download-attachments-custom input output:
    cargo run -- download-attachments --input {{input}} --output {{output}}

# Export selected conversations to markdown
export-markdown:
    cargo run -- export-markdown

# Export selected conversations to markdown with custom paths
export-markdown-custom conversations users channels output:
    cargo run -- export-markdown --conversations {{conversations}} --users {{users}} --channels {{channels}} --output {{output}}

# Export custom emojis
export-emojis:
    cargo run -- export-emojis

# Export custom emojis with custom paths
export-emojis-custom output folder:
    cargo run -- export-emojis --output {{output}} --folder {{folder}}
