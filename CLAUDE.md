# Slack Utils

## Objective

A set of utilities to interact with slack archives

## Technologies

- **ratatui** - Terminal UI framework for building rich terminal interfaces
- **thiserror** - Derive macro for implementing std::error::Error

## Developer Workflow

All to logic should be in lib.rs and modules, the main.rs should only import functionality from lib and call it.

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
