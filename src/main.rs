use clap::Parser;
use slack_utils::{Cli, Commands};

#[tokio::main]
async fn main() {
    // Initialize rustls crypto provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Ui => slack_utils::run_ui(),
        Commands::WorkWeek => {
            slack_utils::run_work_week();
            Ok(())
        }
        Commands::ExportConversations { from, to, output, format } => {
            slack_utils::run_export_conversations(from, to, &output, &format).await
        }
        Commands::ExportConversationsWeek { year, week, output, format } => {
            slack_utils::run_export_conversations_week(year, week, &output, &format).await
        }
        Commands::ArchiveRange { from_year, from_week, to_year, to_week, output } => {
            slack_utils::run_archive_range(from_year, from_week, to_year, to_week, &output).await
        }
        Commands::ExportUsers { output, format } => {
            slack_utils::run_export_users(&output, &format).await
        }
        Commands::ExportChannels { output, format } => {
            slack_utils::run_export_channels(&output, &format).await
        }
        Commands::DownloadAttachments { input, output } => {
            slack_utils::run_download_attachments(&input, &output)
        }
        Commands::ExportMarkdown {
            conversations,
            users,
            channels,
            output,
            formatter_script,
            backslash_line_breaks,
        } => slack_utils::run_export_markdown(&conversations, &users, &channels, &output, formatter_script.as_deref(), backslash_line_breaks),
        Commands::ExportEmojis { output, folder } => {
            slack_utils::run_export_emojis(&output, &folder).await
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
            slack_utils::run_import_index_meilisearch(&input, &url, &api_key, &index_name, clear)
                .await
        }
        Commands::QueryMeilisearch {
            query,
            url,
            api_key,
            index_name,
            limit,
        } => slack_utils::run_query_meilisearch(&url, &api_key, &index_name, &query, limit).await,
        Commands::MdToHtml {
            input,
            output,
            gfm,
            autolink,
            code_indented,
            code_fenced,
            definition,
            frontmatter,
            gfm_autolink_literal,
            gfm_footnote_definition,
            gfm_label_start_footnote,
            gfm_strikethrough,
            gfm_table,
            gfm_task_list_item,
            hard_break_escape,
            hard_break_trailing,
            html_flow,
            html_text,
            label_end,
            label_start_image,
            label_start_link,
            list_item,
            math_flow,
            math_text,
            thematic_break,
            gfm_strikethrough_single_tilde,
            math_text_single_dollar,
        } => {
            let options = if gfm {
                slack_utils::md_to_html::MdToHtmlOptions::gfm()
            } else {
                slack_utils::md_to_html::MdToHtmlOptions {
                    gfm,
                    autolink,
                    code_indented,
                    code_fenced,
                    definition,
                    frontmatter,
                    gfm_autolink_literal,
                    gfm_footnote_definition,
                    gfm_label_start_footnote,
                    gfm_strikethrough,
                    gfm_table,
                    gfm_task_list_item,
                    hard_break_escape,
                    hard_break_trailing,
                    html_flow,
                    html_text,
                    label_end,
                    label_start_image,
                    label_start_link,
                    list_item,
                    math_flow,
                    math_text,
                    thematic_break,
                    gfm_strikethrough_single_tilde,
                    math_text_single_dollar,
                }
            };
            slack_utils::run_md_to_html(&input, output.as_deref(), &options)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
