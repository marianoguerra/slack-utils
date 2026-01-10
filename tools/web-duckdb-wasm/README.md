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

## URL Query Parameters

You can pre-fill the date range and auto-load conversations by passing query parameters in the URL. This is useful for bookmarking specific time ranges or sharing links.

### Week-based Parameters

Load data by ISO week number:

| Parameter | Required | Description |
|-----------|----------|-------------|
| `fromYear` | Yes | Start year (e.g., `2024`) |
| `fromWeek` | Yes | Start ISO week number (1-53) |
| `toYear` | No | End year (defaults to `fromYear`) |
| `toWeek` | No | End ISO week number (defaults to `fromWeek`) |

**Examples:**

```
# Load a single week (week 42 of 2024)
?fromYear=2024&fromWeek=42

# Load multiple weeks in the same year (weeks 40-45 of 2024)
?fromYear=2024&fromWeek=40&toWeek=45

# Load weeks across year boundary (week 50 of 2024 to week 2 of 2025)
?fromYear=2024&fromWeek=50&toYear=2025&toWeek=2
```

### Date-based Parameters

Load data by specific dates:

| Parameter | Required | Description |
|-----------|----------|-------------|
| `fromDate` | Yes | Start date in `YYYY-MM-DD` format |
| `toDate` | Yes | End date in `YYYY-MM-DD` format |

**Examples:**

```
# Load October 2024
?fromDate=2024-10-01&toDate=2024-10-31

# Load Q4 2024
?fromDate=2024-10-01&toDate=2024-12-31
```

### Behavior

When query parameters are present:
1. The date inputs are pre-filled with the calculated date range
2. After users and channels finish loading, conversations are automatically loaded
3. If no query parameters are provided, the date range defaults to the last 2 months (no auto-load)

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
      threads.parquet
    week=02/
      threads.parquet
  year=2025/
    week=01/
      threads.parquet
```

## Requirements

- Modern browser with WebAssembly support
- Bun (for the development servers)
- Parquet files exported from slack-utils
