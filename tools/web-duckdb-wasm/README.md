# Web DuckDB Query Tool

A browser-based SQL query tool for Slack parquet exports using DuckDB-WASM.

## Features

- Auto-loads users and channels on startup
- Date range picker to load conversation data (defaults to last 2 months)
- Run SQL queries against conversations, users, and channels
- Predefined sample queries for common statistics
- All queries run locally in WebAssembly - no data leaves your browser

## Usage

### Start the server

```bash
cd tools/web-duckdb-wasm
bun install
bun run serve -- --path /path/to/parquet/files
```

Or from the project root:

```bash
cd tools/web-duckdb-wasm && bun run serve -- --path ../..
```

### CLI Options

```
Usage: bun serve.js [options]

Options:
  -p, --path <path>   Base path containing parquet files (default: current directory)
  --port <port>       Server port (default: 3000)
  -h, --help          Show this help message

Required files in base path:
  - users.parquet
  - channels.parquet
  - conversations/    (directory with year=*/week=*/*.parquet structure)
```

### Export data first

Before using the web tool, export your Slack data in parquet format:

```bash
slack-utils export-users --format parquet
slack-utils export-channels --format parquet
slack-utils export-conversations --format parquet
```

Then open http://localhost:3000 in your browser.

### Run queries

1. **Users and Channels** load automatically on page load
2. **Conversations**: Select a date range and click "Load Conversations"
3. Use the dropdown to select a predefined sample query, or write your own SQL

Tables available:
- `users` - User data
- `channels` - Channel data
- `conversations` - Message data (after loading)

Press `Ctrl+Enter` (or `Cmd+Enter` on Mac) to run the query.

## Sample Queries

The tool includes predefined queries for:

### Overview Stats
- Total users (bots, admins count)
- Total channels (archived, private count)
- Total messages overview

### Channel Stats
- Top channels by member count
- Top channels by message count

### User Activity
- Most active users
- List of human (non-bot) users

### Time-based Stats
- Messages by week
- Messages by date

### Thread Activity
- Top threads by reply count
- Thread statistics

### Content Search
- Search messages by keyword
- Recent messages

## Requirements

- Modern browser with WebAssembly support
- Bun (for the server)
- Parquet files exported from slack-utils
