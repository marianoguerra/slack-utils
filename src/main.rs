use clap::Parser;
use slack_utils::{Cli, Commands};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Ui => slack_utils::run_ui(),
        Commands::ExportConversations { from, to, output } => {
            slack_utils::run_export_conversations_async(from, to, &output).await
        }
        Commands::ExportUsers { output } => slack_utils::run_export_users_async(&output).await,
        Commands::ExportChannels { output } => slack_utils::run_export_channels_async(&output).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
