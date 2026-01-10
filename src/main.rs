use clap::Parser;
use slack_utils::{Cli, Commands};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Ui => slack_utils::run_ui(),
        Commands::ExportConversations { from, to, output, format } => {
            slack_utils::run_export_conversations_async(from, to, &output, &format).await
        }
        Commands::ExportUsers { output, format } => {
            slack_utils::run_export_users_async(&output, &format).await
        }
        Commands::ExportChannels { output, format } => {
            slack_utils::run_export_channels_async(&output, &format).await
        }
        Commands::DownloadAttachments { input, output } => {
            slack_utils::run_download_attachments(&input, &output)
        }
        Commands::ExportMarkdown {
            conversations,
            users,
            channels,
            output,
        } => slack_utils::run_export_markdown(&conversations, &users, &channels, &output),
        Commands::ExportEmojis { output, folder } => {
            slack_utils::run_export_emojis_async(&output, &folder).await
        }
        Commands::ExportIndex {
            conversations,
            users,
            channels,
            output,
        } => slack_utils::run_export_index(&conversations, &users, &channels, &output),
        Commands::ImportIndexMeilisearch {
            input,
            url,
            api_key,
            index_name,
            clear,
        } => {
            slack_utils::run_import_index_meilisearch_async(&input, &url, &api_key, &index_name, clear)
                .await
        }
        Commands::QueryMeilisearch {
            query,
            url,
            api_key,
            index_name,
            limit,
        } => slack_utils::run_query_meilisearch_async(&url, &api_key, &index_name, &query, limit).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
