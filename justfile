# Slack Utils justfile

# File path defaults
conversations_file := "conversations.json"
selected_conversations_file := "selected-conversations.json"
users_file := "users.json"
channels_file := "channels.json"
attachments_dir := "attachments"
emojis_file := "emojis.json"
emojis_dir := "emojis"
index_file := "conversation-index.json"
markdown_file := "selected-conversations.md"

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
export-conversations output=conversations_file:
    #!/usr/bin/env bash
    from=$(date -d '7 days ago' +%Y-%m-%d)
    to=$(date +%Y-%m-%d)
    cargo run -- export-conversations --from "$from" --to "$to" --output {{output}}

# Export conversations with custom date range
export-conversations-range from to output=conversations_file:
    cargo run -- export-conversations --from {{from}} --to {{to}} --output {{output}}

# Export users
export-users output=users_file:
    cargo run -- export-users --output {{output}}

# Export channels
export-channels output=channels_file:
    cargo run -- export-channels --output {{output}}

# Download attachments from conversations
download-attachments input=conversations_file output=attachments_dir:
    cargo run -- download-attachments --input {{input}} --output {{output}}

# Export selected conversations to markdown
export-markdown conversations=selected_conversations_file users=users_file channels=channels_file output=markdown_file:
    cargo run -- export-markdown --conversations {{conversations}} --users {{users}} --channels {{channels}} --output {{output}}

# Export custom emojis
export-emojis output=emojis_file folder=emojis_dir:
    cargo run -- export-emojis --output {{output}} --folder {{folder}}

# Export conversations to searchable index
export-index conversations=conversations_file users=users_file channels=channels_file output=index_file:
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
