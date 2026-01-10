# Slack Utils justfile

# File path defaults (without extensions for format flexibility)
conversations_path := "conversations"
selected_conversations_file := "selected-conversations.json"
users_path := "users"
channels_path := "channels"
attachments_dir := "attachments"
emojis_file := "emojis.json"
emojis_dir := "emojis"
index_file := "conversation-index.json"
markdown_file := "selected-conversations.md"

# Format default (json or parquet)
default_format := "json"

# Meilisearch defaults
ms_url := "http://localhost:7700"
ms_index := "slack"

# Default recipe to list available targets
default:
    @just --list

# Launch the interactive TUI
ui:
    cargo run -- ui

# Export conversations from the last 7 days
export-conversations output=conversations_path format=default_format:
    #!/usr/bin/env bash
    from=$(date -d '7 days ago' +%Y-%m-%d)
    to=$(date +%Y-%m-%d)
    cargo run -- export-conversations --from "$from" --to "$to" --output {{output}} --format {{format}}

# Export conversations with custom date range
export-conversations-range from to output=conversations_path format=default_format:
    cargo run -- export-conversations --from {{from}} --to {{to}} --output {{output}} --format {{format}}

# Export conversations for current work week (defaults to current year and week)
export-conversations-week output=conversations_path format=default_format:
    cargo run -- export-conversations-week --output {{output}} --format {{format}}

# Export conversations for specific work week
export-conversations-week-custom year week output=conversations_path format=default_format:
    cargo run -- export-conversations-week --year {{year}} --week {{week}} --output {{output}} --format {{format}}

# Export users
export-users output=users_path format=default_format:
    cargo run -- export-users --output {{output}} --format {{format}}

# Export channels
export-channels output=channels_path format=default_format:
    cargo run -- export-channels --output {{output}} --format {{format}}

# Download attachments from conversations
download-attachments input="conversations.json" output=attachments_dir:
    cargo run -- download-attachments --input {{input}} --output {{output}}

# Export selected conversations to markdown
export-markdown conversations=selected_conversations_file users="users.json" channels="channels.json" output=markdown_file:
    cargo run -- export-markdown --conversations {{conversations}} --users {{users}} --channels {{channels}} --output {{output}}

# Export custom emojis
export-emojis output=emojis_file folder=emojis_dir:
    cargo run -- export-emojis --output {{output}} --folder {{folder}}

# Export conversations to searchable index
export-index conversations="conversations.json" users="users.json" channels="channels.json" output=index_file:
    cargo run -- export-index --conversations {{conversations}} --users {{users}} --channels {{channels}} --output {{output}}

# Import index to Meilisearch
import-meilisearch api_key input=index_file url=ms_url index_name=ms_index:
    cargo run -- import-index-meilisearch --input {{input}} --url {{url}} --api-key {{api_key}} --index-name {{index_name}}

# Import index to Meilisearch and clear existing data
import-meilisearch-clear api_key input=index_file url=ms_url index_name=ms_index:
    cargo run -- import-index-meilisearch --input {{input}} --url {{url}} --api-key {{api_key}} --index-name {{index_name}} --clear

# Query Meilisearch index
query-meilisearch query api_key url=ms_url index_name=ms_index:
    cargo run -- query-meilisearch "{{query}}" --url {{url}} --api-key {{api_key}} --index-name {{index_name}}

# Start Meilisearch server
start-meilisearch:
    ./meilisearch --master-key $MS_MASTER_KEY
