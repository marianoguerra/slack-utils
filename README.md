# slack-utils

A set of utilities to interact with Slack archives.

## Requirements

- Rust
- Slack API token (set as `SLACK_TOKEN` environment variable)
- Meilisearch (optional, for search features)

## Installation

```bash
cargo build --release
```

## Usage

### Interactive TUI

```bash
slack-utils ui
```

### CLI Commands

**Export data from Slack:**

```bash
# Export users
slack-utils export-users

# Export channels
slack-utils export-channels

# Export conversations in a date range
slack-utils export-conversations --from 2024-01-01 --to 2024-01-31

# Export custom emojis
slack-utils export-emojis
```

**Process exported data:**

```bash
# Download attachments from conversations
slack-utils download-attachments --input conversations.json --output attachments/

# Export conversations to markdown
slack-utils export-markdown --conversations selected-conversations.json --output output.md

# Export conversations to searchable index
slack-utils export-index --conversations conversations.json --output conversation-index.json
```

**Meilisearch integration:**

```bash
# Import index to Meilisearch
slack-utils import-index-meilisearch --url http://localhost:7700 --api-key KEY --index-name slack

# Import and clear existing index
slack-utils import-index-meilisearch --url http://localhost:7700 --api-key KEY --index-name slack --clear

# Search the index
slack-utils query-meilisearch "search term" --url http://localhost:7700 --api-key KEY --index-name slack
```

## Configuration

Settings are saved to `settings.toml` in the current directory. Meilisearch connection details are persisted automatically after import.
