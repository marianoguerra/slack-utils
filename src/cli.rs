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

        /// Output file path
        #[arg(short, long, default_value = "conversations.json")]
        output: String,
    },

    /// Export users
    ExportUsers {
        /// Output file path
        #[arg(short, long, default_value = "users.json")]
        output: String,
    },

    /// Export channels
    ExportChannels {
        /// Output file path
        #[arg(short, long, default_value = "channels.json")]
        output: String,
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
}
