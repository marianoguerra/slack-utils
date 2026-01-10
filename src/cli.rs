use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "slack-utils")]
#[command(about = "A set of utilities to interact with Slack archives")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Launch the interactive TUI
    Ui,

    /// Export conversations in a date range
    ExportConversations {
        /// Start date (YYYY-MM-DD), defaults to 7 days ago
        #[arg(short, long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD), defaults to today
        #[arg(short, long)]
        to: Option<String>,

        /// Output path (without extension for json, directory path for parquet)
        #[arg(short, long, default_value = "conversations")]
        output: String,

        /// Output format (json or parquet)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Export users
    ExportUsers {
        /// Output path (without extension)
        #[arg(short, long, default_value = "users")]
        output: String,

        /// Output format (json or parquet)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Export channels
    ExportChannels {
        /// Output path (without extension)
        #[arg(short, long, default_value = "channels")]
        output: String,

        /// Output format (json or parquet)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Download attachments from a conversations file
    DownloadAttachments {
        /// Input conversations file path
        #[arg(short, long, default_value = "conversations.json")]
        input: String,

        /// Output directory path
        #[arg(short, long, default_value = "attachments")]
        output: String,
    },

    /// Export selected conversations to markdown
    ExportMarkdown {
        /// Input selected conversations file path
        #[arg(short, long, default_value = "selected-conversations.json")]
        conversations: String,

        /// Users JSON file path
        #[arg(short, long, default_value = "users.json")]
        users: String,

        /// Channels JSON file path
        #[arg(long, default_value = "channels.json")]
        channels: String,

        /// Output markdown file path
        #[arg(short, long, default_value = "selected-conversations.md")]
        output: String,
    },

    /// Export custom emojis from Slack
    ExportEmojis {
        /// Output JSON file path for emoji data
        #[arg(short, long, default_value = "emojis.json")]
        output: String,

        /// Folder to download emoji images
        #[arg(short, long, default_value = "emojis")]
        folder: String,
    },

    /// Export conversations to a searchable index
    ExportIndex {
        /// Input conversations file path
        #[arg(short, long, default_value = "conversations.json")]
        conversations: String,

        /// Users JSON file path
        #[arg(short, long, default_value = "users.json")]
        users: String,

        /// Channels JSON file path
        #[arg(long, default_value = "channels.json")]
        channels: String,

        /// Output index JSON file path
        #[arg(short, long, default_value = "conversation-index.json")]
        output: String,
    },

    /// Import index to Meilisearch
    ImportIndexMeilisearch {
        /// Input index JSON file path
        #[arg(short, long, default_value = "conversation-index.json")]
        input: String,

        /// Meilisearch server URL
        #[arg(short, long, default_value = "http://localhost:7700")]
        url: String,

        /// Meilisearch API key
        #[arg(short, long, default_value = "")]
        api_key: String,

        /// Meilisearch index name
        #[arg(short = 'n', long, default_value = "conversations")]
        index_name: String,

        /// Clear index before import (uses swap operation)
        #[arg(short, long, default_value = "false")]
        clear: bool,
    },

    /// Query Meilisearch index
    QueryMeilisearch {
        /// Search query
        query: String,

        /// Meilisearch server URL
        #[arg(short, long, default_value = "http://localhost:7700")]
        url: String,

        /// Meilisearch API key
        #[arg(short, long, default_value = "")]
        api_key: String,

        /// Meilisearch index name
        #[arg(short = 'n', long, default_value = "conversations")]
        index_name: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}
