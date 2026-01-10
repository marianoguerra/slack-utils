# Web DuckDB Query Tool

A browser-based SQL query tool for Slack parquet exports using DuckDB-WASM.

## Features

- Load parquet files directly in the browser
- Run SQL queries against conversations, users, and channels
- Predefined sample queries for common statistics
- No server-side processing - all queries run locally in WebAssembly

## Usage

### Start the development server

```bash
cd tools/web-duckdb-wasm
bun install
bun run dev
```

Then open http://localhost:3000 in your browser.

### Load parquet files

1. **Conversations**: Select one or more parquet files from `conversations/year=*/week=*/` directories
2. **Users**: Load `users.parquet`
3. **Channels**: Load `channels.parquet`

### Run queries

Use the dropdown to select a predefined sample query, or write your own SQL. Tables are named:
- `conversations` - Message data
- `users` - User data
- `channels` - Channel data

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
- Bun (for development server)
