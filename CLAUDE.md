# Slack Utils

## Objective

A set of utilities to interact with slack archives

## Technologies

- **ratatui** - Terminal UI framework for building rich terminal interfaces
- **thiserror** - Derive macro for implementing std::error::Error
- **duckdb** - Optional dependency for SQL queries on parquet exports (feature-gated)
- **axum** - Optional dependency for HTTP server (feature-gated)
- **bun** - Use bun for any JavaScript/TypeScript tasks

## slack-utils-duckdb Binary

A separate binary for querying parquet exports using DuckDB. Built with the `duckdb` feature flag.

### Building

```bash
cargo build --features duckdb --bin slack-utils-duckdb
```

### Usage

```bash
# Query with default parquet path (conversations/year=*/week=*/*.parquet)
slack-utils-duckdb query "SELECT * FROM data LIMIT 10"

# Specify a custom parquet path
slack-utils-duckdb query "SELECT * FROM data" --parquet users.parquet
slack-utils-duckdb query "SELECT * FROM data" --parquet channels.parquet
```

### Sample Queries

```bash
# Count messages per channel
slack-utils-duckdb query "SELECT channel_name, COUNT(*) as msg_count FROM data GROUP BY channel_name ORDER BY msg_count DESC"

# Find messages from a specific week (uses Hive partition filtering)
slack-utils-duckdb query "SELECT * FROM data WHERE year = 2024 AND week = 42 LIMIT 20"

# Search for messages containing a keyword
slack-utils-duckdb query "SELECT channel_name, user, text FROM data WHERE text LIKE '%deploy%' LIMIT 50"

# Get message counts by user
slack-utils-duckdb query "SELECT user, COUNT(*) as msg_count FROM data GROUP BY user ORDER BY msg_count DESC LIMIT 20"

# Count thread replies
slack-utils-duckdb query "SELECT COUNT(*) as replies FROM data WHERE is_reply = true"

# Messages per day for a date range
slack-utils-duckdb query "SELECT date, COUNT(*) FROM data GROUP BY date ORDER BY date"

# Query users parquet
slack-utils-duckdb query "SELECT name, real_name, email FROM data WHERE is_bot = false" --parquet users.parquet

# Query channels parquet
slack-utils-duckdb query "SELECT name, num_members FROM data WHERE is_archived = false ORDER BY num_members DESC" --parquet channels.parquet
```

## slack-archive-server Binary

An HTTP server for serving Slack archive parquet files. Built with the `server` feature flag.

### Building

```bash
cargo build --features server --bin slack-archive-server
```

### Configuration

The server requires a TOML configuration file. See `resources/sample-server-config.toml` for a fully documented example.

```toml
[server]
host = "127.0.0.1"
port = 8080
# static_assets = "./static"  # Optional: serve static files

[slack-archive]
base_path = "./archive"
```

The `base_path` should point to a directory with this structure:

```
archive/
├── users.parquet
├── channels.parquet
└── conversations/
    └── year=YYYY/
        └── week=WW/
            └── threads.parquet
```

### Usage

```bash
# Using cargo
cargo run --features server --bin slack-archive-server -- serve config.toml

# Using just
just run-server config.toml

# Or build first, then run directly
just build-server
./target/debug/slack-archive-server serve config.toml
```

### API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /archive/users` | Returns `users.parquet` file |
| `GET /archive/channels` | Returns `channels.parquet` file |
| `GET /archive/threads-in-range?from=YYYY-MM-DD&to=YYYY-MM-DD` | Lists available year/week partitions in date range |
| `GET /archive/threads?year=YYYY&week=WW` | Returns `threads.parquet` for specific week |
| `POST /archive/search?query=<text>&limit=<n>` | Search messages via Meilisearch (requires config) |

### Example API Calls

```bash
# Get users parquet file
curl -O http://localhost:8080/archive/users

# Get channels parquet file
curl -O http://localhost:8080/archive/channels

# List available weeks in January 2024
curl "http://localhost:8080/archive/threads-in-range?from=2024-01-01&to=2024-01-31"
# Response: {"available":[{"year":2024,"week":1},{"year":2024,"week":2},...]}

# Get threads for week 3 of 2024
curl -O "http://localhost:8080/archive/threads?year=2024&week=3"

# Search for messages (requires meilisearch config)
curl -X POST "http://localhost:8080/archive/search?query=deployment&limit=20"
# Response: {"hits":[...],"processing_time_ms":5,"estimated_total_hits":42}
```

### Smoke Test

```bash
just server-smoke-test
```

## Developer Workflow

All to logic should be in lib.rs and modules, the main.rs should only import functionality from lib and call it.

Don't use unwrap or expect or any other functionality that causes a panic at runtime outside of tests, always handle error cases with proper error handling.

Always keep the tui/cli README.md smoke-test target and justfile in sync when adding new features and fields/options

Reuse core logic for the tui and cli.

When implementing tasks that may take more than 1 second add a loading screen to the tui and progress to the cli, make sure the progress report functionality of the cli doesn't appear in the tui when reusing core logic.

Prefer functions over macros unless macros are strictly necessary (e.g., for compile-time code generation that cannot be achieved with generics or closures).

## Error Handling

Use `thiserror` for all error handling:

- Create dedicated error enums with variants for each error kind
- Wrap source errors using `#[from]` or `#[source]`
- Add `#[error("...")]` annotations with descriptive messages that include context
- Avoid generic catch-all variants; be specific about what can fail

Example:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file at {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid config format: {0}")]
    InvalidFormat(String),
}
```

## Workflow

After each change, run:

```bash
cargo test
cargo clippy
```

When asked to update deps, run the following command to see the newest version, ask if the user wants Compat or Latest update if not specified:

```bash
cargo outdated -R
```
