# Web DuckDB Query Tool

A browser-based SQL query tool for Slack parquet exports using DuckDB-WASM.

## Features

- Auto-loads users and channels on startup
- Date range picker to load conversation data (defaults to last 2 months)
- Run SQL queries against conversations, users, and channels
- Predefined sample queries for common statistics
- All queries run locally in WebAssembly - no data leaves your browser

## Two Server Modes

### 1. API Server (`serve.js`)

Backend provides API endpoints to list and serve parquet files.

```bash
bun run serve -- --path /path/to/parquet/files
```

Open http://localhost:3000

### 2. Static File Server (`static-file-server.js`)

Simple static file server - frontend discovers files by trying to fetch them based on Hive partition structure (ignoring 404s). Useful for deploying to any static file host.

```bash
bun run serve-static -- --path /path/to/parquet/files
```

Open http://localhost:3000

## Usage

### Install dependencies

```bash
cd tools/web-duckdb-wasm
bun install
```

### Start a server

```bash
# API server (recommended for development)
bun run serve -- --path ../..

# Static file server (for static hosting scenarios)
bun run serve-static -- --path ../..
```

### CLI Options

Both servers accept the same options:

```
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

## File Structure

The static file mode expects the following Hive partition structure:

```
/users.parquet
/channels.parquet
/conversations/
  year=2024/
    week=01/
      conversations.parquet
    week=02/
      conversations.parquet
  year=2025/
    week=01/
      conversations.parquet
```

## Requirements

- Modern browser with WebAssembly support
- Bun (for the development servers)
- Parquet files exported from slack-utils
