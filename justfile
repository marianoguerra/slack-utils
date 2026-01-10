# Slack Utils justfile

# Meilisearch defaults (override with: just --set ms_url "..." target)
ms_url := "http://localhost:7700"
ms_index := "slack"

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

# Export conversations to searchable index
export-index:
    cargo run -- export-index

# Export conversations to searchable index with custom paths
export-index-custom conversations users channels output:
    cargo run -- export-index --conversations {{conversations}} --users {{users}} --channels {{channels}} --output {{output}}

# Import index to Meilisearch
import-meilisearch api_key url=ms_url index_name=ms_index:
    cargo run -- import-index-meilisearch --url {{url}} --api-key {{api_key}} --index-name {{index_name}}

# Import index to Meilisearch with custom input path
import-meilisearch-custom input api_key url=ms_url index_name=ms_index:
    cargo run -- import-index-meilisearch --input {{input}} --url {{url}} --api-key {{api_key}} --index-name {{index_name}}

# Import index to Meilisearch and clear existing data
import-meilisearch-clear api_key url=ms_url index_name=ms_index:
    cargo run -- import-index-meilisearch --url {{url}} --api-key {{api_key}} --index-name {{index_name}} --clear

# Query Meilisearch index
query-meilisearch query api_key url=ms_url index_name=ms_index:
    cargo run -- query-meilisearch "{{query}}" --url {{url}} --api-key {{api_key}} --index-name {{index_name}}

# Start Meilisearch server
start-meilisearch:
  ./meilisearch --master-key $MS_MASTER_KEY
