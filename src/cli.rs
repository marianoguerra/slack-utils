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

    /// Export conversations for a specific ISO work week
    ExportConversationsWeek {
        /// ISO year (defaults to current year)
        #[arg(short, long)]
        year: Option<i32>,

        /// ISO week number 1-53 (defaults to current week)
        #[arg(short, long)]
        week: Option<u32>,

        /// Output path (without extension for json, directory path for parquet)
        #[arg(short, long, default_value = "conversations")]
        output: String,

        /// Output format (json or parquet)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Archive conversations for a range of ISO weeks (parquet format)
    ArchiveRange {
        /// Start ISO year (defaults to current year)
        #[arg(long, default_value_t = 0)]
        from_year: i32,

        /// Start ISO week number 1-53 (defaults to current week)
        #[arg(long, default_value_t = 0)]
        from_week: u32,

        /// End ISO year (defaults to from-year)
        #[arg(long)]
        to_year: Option<i32>,

        /// End ISO week number 1-53 (defaults to from-week)
        #[arg(long)]
        to_week: Option<u32>,

        /// Output directory path for parquet files
        #[arg(short, long, default_value = "conversations")]
        output: String,
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

        /// External formatter script path (overrides settings.toml)
        #[arg(long)]
        formatter_script: Option<String>,

        /// Convert newlines to backslash + newline for hard line breaks in markdown
        #[arg(long)]
        backslash_line_breaks: bool,
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
        #[arg(short = 'n', long, default_value = "slack")]
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
        #[arg(short = 'n', long, default_value = "slack")]
        index_name: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Convert Markdown file to HTML
    MdToHtml {
        /// Input markdown file path
        input: String,

        /// Output HTML file path (defaults to input with .html extension)
        #[arg(short, long)]
        output: Option<String>,

        /// Use GFM (GitHub Flavored Markdown) preset
        #[arg(long)]
        gfm: bool,

        /// Enable autolinks (URLs become links automatically) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        autolink: bool,

        /// Enable code (indented) blocks [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        code_indented: bool,

        /// Enable code (fenced) blocks [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        code_fenced: bool,

        /// Enable definition lists [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        definition: bool,

        /// Enable frontmatter (YAML metadata) [default: false]
        #[arg(long)]
        frontmatter: bool,

        /// Enable GFM autolink literals [default: false]
        #[arg(long)]
        gfm_autolink_literal: bool,

        /// Enable GFM footnote definitions [default: false]
        #[arg(long)]
        gfm_footnote_definition: bool,

        /// Enable GFM label start footnote [default: false]
        #[arg(long)]
        gfm_label_start_footnote: bool,

        /// Enable GFM strikethrough [default: false]
        #[arg(long)]
        gfm_strikethrough: bool,

        /// Enable GFM tables [default: false]
        #[arg(long)]
        gfm_table: bool,

        /// Enable GFM task list items [default: false]
        #[arg(long)]
        gfm_task_list_item: bool,

        /// Enable hard break (escape) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        hard_break_escape: bool,

        /// Enable hard break (trailing) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        hard_break_trailing: bool,

        /// Enable HTML (flow) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        html_flow: bool,

        /// Enable HTML (text) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        html_text: bool,

        /// Enable label end [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        label_end: bool,

        /// Enable label start (image) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        label_start_image: bool,

        /// Enable label start (link) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        label_start_link: bool,

        /// Enable list items [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        list_item: bool,

        /// Enable math (flow) [default: false]
        #[arg(long)]
        math_flow: bool,

        /// Enable math (text) [default: false]
        #[arg(long)]
        math_text: bool,

        /// Enable thematic break (---) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        thematic_break: bool,

        /// Use single tilde for strikethrough (~text~) [default: false]
        #[arg(long)]
        gfm_strikethrough_single_tilde: bool,

        /// Use single dollar for math ($x$) [default: true]
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        math_text_single_dollar: bool,
    },
}
