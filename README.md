# slack-utils

A set of utilities to interact with Slack archives.

## Requirements

- Rust
- Slack API token (set as `SLACK_TOKEN` environment variable)
- Meilisearch (optional, for search features)
- just (optional, for running tasks)

## Installation

```bash
cargo build --release
```

## Usage

### Interactive TUI

```bash
slack-utils ui
# or
just ui
```

### Using just

All targets have sensible defaults and can be run without arguments:

```bash
# Export data from Slack
just export-users
just export-channels
just export-conversations
just export-emojis

# Process exported data
just download-attachments
just export-markdown
just export-index

# Meilisearch (api_key required)
just import-meilisearch "YOUR_API_KEY"
just import-meilisearch-clear "YOUR_API_KEY"
just query-meilisearch "search term" "YOUR_API_KEY"
```

Override defaults by passing arguments:

```bash
# Custom output file
just export-users custom-users.json

# Custom date range
just export-conversations-range 2024-01-01 2024-01-31

# Custom paths for markdown export
just export-markdown my-conversations.json users.json channels.json output.md

# Custom Meilisearch settings
just query-meilisearch "search" "API_KEY" "http://other:7700" "my-index"
```

Override defaults via variables:

```bash
just --set ms_url "http://other:7700" import-meilisearch "API_KEY"
just --set conversations_file "my-data.json" export-index
```

### CLI Commands

**Export data from Slack:**

```bash
slack-utils export-users
slack-utils export-channels
slack-utils export-conversations --from 2024-01-01 --to 2024-01-31
slack-utils export-emojis
```

**Archive conversations by week range (parquet format):**

```bash
# Archive a single week (defaults to current week if not specified)
slack-utils archive-range --from-year 2024 --from-week 42

# Archive multiple weeks in the same year
slack-utils archive-range --from-year 2024 --from-week 1 --to-week 52

# Archive across year boundary
slack-utils archive-range --from-year 2024 --from-week 50 --to-year 2025 --to-week 10

# Custom output directory
slack-utils archive-range --from-year 2024 --from-week 1 --to-week 10 --output ./my-archive
```

Features:
- Exports each week to `year=YYYY/week=WW/threads.parquet`
- Skips weeks that already have parquet files (incremental archiving)
- Handles Slack API rate limits with automatic retry (up to 5 attempts, uses Retry-After header)
- Progress reporting shows current week, message counts, and rate limit waits

**Process exported data:**

```bash
slack-utils download-attachments --input conversations.json --output attachments/
slack-utils export-markdown --conversations selected-conversations.json --output output.md
slack-utils export-index --conversations conversations.json --output conversation-index.json
```

**Meilisearch integration:**

```bash
slack-utils import-index-meilisearch --url http://localhost:7700 --api-key KEY --index-name slack
slack-utils import-index-meilisearch --url http://localhost:7700 --api-key KEY --index-name slack --clear
slack-utils query-meilisearch "search term" --url http://localhost:7700 --api-key KEY --index-name slack
```

## Configuration

Settings are saved to `settings.toml` in the current directory. Meilisearch connection details are persisted automatically after import.

Default file paths used by justfile:

| Variable | Default |
|----------|---------|
| `conversations_file` | `conversations.json` |
| `selected_conversations_file` | `selected-conversations.json` |
| `users_file` | `users.json` |
| `channels_file` | `channels.json` |
| `attachments_dir` | `attachments` |
| `emojis_file` | `emojis.json` |
| `emojis_dir` | `emojis` |
| `index_file` | `conversation-index.json` |
| `markdown_file` | `selected-conversations.md` |
| `ms_url` | `http://localhost:7700` |
| `ms_index` | `slack` |
