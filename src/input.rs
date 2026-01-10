use std::sync::mpsc;
use std::thread;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::app::App;
use crate::slack;
use crate::settings::Settings;
use crate::ui::types::{
    AsyncResult, ConvExportField, DownloadAttachmentsField, EditConvPathField,
    EditableChannelList, ExportEmojisField, ExportIndexField, ExportTask, ImportMeilisearchField,
    MarkdownExportField, MenuItem, QueryMeilisearchField, Screen,
};
use crate::widgets::TextInput;

pub fn handle_input(app: &mut App, key: KeyEvent) {
    match &mut app.screen {
        Screen::MainMenu => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => app.menu_previous(),
            KeyCode::Down | KeyCode::Char('j') => app.menu_next(),
            KeyCode::Enter => {
                let item = app.selected_menu_item();
                match item {
                    MenuItem::ExportConversations => {
                        app.open_export_conversations();
                    }
                    MenuItem::ExportUsers => {
                        app.screen = Screen::ExportUsers {
                            output_path: "users.json".to_string(),
                        };
                    }
                    MenuItem::ExportChannels => {
                        app.screen = Screen::ExportChannels {
                            output_path: "channels.json".to_string(),
                        };
                    }
                    MenuItem::EditConversations => {
                        app.screen = Screen::EditConversationsPathInput {
                            conversations_path: "./conversations.json".to_string(),
                            users_path: "./users.json".to_string(),
                            channels_path: "./channels.json".to_string(),
                            active_field: EditConvPathField::Conversations,
                        };
                    }
                    MenuItem::DownloadAttachments => {
                        app.screen = Screen::DownloadAttachments {
                            conversations_path: "./conversations.json".to_string(),
                            output_path: "./attachments".to_string(),
                            active_field: DownloadAttachmentsField::ConversationsPath,
                        };
                    }
                    MenuItem::SelectedConversationsToMarkdown => {
                        app.screen = Screen::MarkdownExport {
                            conversations_path: "./selected-conversations.json".to_string(),
                            users_path: "./users.json".to_string(),
                            channels_path: "./channels.json".to_string(),
                            output_path: "./selected-conversations.md".to_string(),
                            active_field: MarkdownExportField::Conversations,
                        };
                    }
                    MenuItem::ExportEmojis => {
                        app.screen = Screen::ExportEmojis {
                            output_path: "emojis.json".to_string(),
                            emojis_folder: "emojis".to_string(),
                            active_field: ExportEmojisField::OutputPath,
                        };
                    }
                    MenuItem::ExportIndex => {
                        app.screen = Screen::ExportIndex {
                            conversations_path: "./conversations.json".to_string(),
                            users_path: "./users.json".to_string(),
                            channels_path: "./channels.json".to_string(),
                            output_path: "./conversation-index.json".to_string(),
                            active_field: ExportIndexField::Conversations,
                        };
                    }
                    MenuItem::ImportIndexMeilisearch => {
                        // Load saved settings if available
                        let saved = Settings::load().ok();
                        let ms = saved.as_ref().map(|s| &s.meilisearch);
                        app.screen = Screen::ImportMeilisearch {
                            input_path: "./conversation-index.json".to_string(),
                            url: ms.map(|m| m.url.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "http://localhost:7700".to_string()),
                            api_key: ms.map(|m| m.api_key.clone()).unwrap_or_default(),
                            index_name: ms.map(|m| m.index_name.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "conversations".to_string()),
                            clear: false,
                            active_field: ImportMeilisearchField::Input,
                        };
                    }
                    MenuItem::QueryMeilisearch => {
                        // Load saved settings if available
                        let saved = Settings::load().ok();
                        let ms = saved.as_ref().map(|s| &s.meilisearch);
                        app.screen = Screen::QueryMeilisearch {
                            query: String::new(),
                            url: ms.map(|m| m.url.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "http://localhost:7700".to_string()),
                            api_key: ms.map(|m| m.api_key.clone()).unwrap_or_default(),
                            index_name: ms.map(|m| m.index_name.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "conversations".to_string()),
                            active_field: QueryMeilisearchField::Query,
                            results: None,
                            result_state: ListState::default(),
                            error: None,
                        };
                    }
                    MenuItem::Exit => app.should_quit = true,
                }
            }
            _ => {}
        },
        Screen::ExportConversations {
            from_date,
            to_date,
            output_path,
            active_field,
            channel_selection,
            loading_channels,
        } => {
            if *loading_channels {
                return;
            }

            match key.code {
                KeyCode::Esc => app.screen = Screen::MainMenu,
                KeyCode::Tab => {
                    *active_field = match active_field {
                        ConvExportField::FromDate => ConvExportField::ToDate,
                        ConvExportField::ToDate => ConvExportField::OutputPath,
                        ConvExportField::OutputPath => ConvExportField::Channels,
                        ConvExportField::Channels => ConvExportField::FromDate,
                    };
                }
                KeyCode::BackTab => {
                    *active_field = match active_field {
                        ConvExportField::FromDate => ConvExportField::Channels,
                        ConvExportField::ToDate => ConvExportField::FromDate,
                        ConvExportField::OutputPath => ConvExportField::ToDate,
                        ConvExportField::Channels => ConvExportField::OutputPath,
                    };
                }
                KeyCode::Char('r') if *active_field == ConvExportField::Channels => {
                    *loading_channels = true;
                    *channel_selection = None;
                    let (tx, rx) = mpsc::channel();
                    app.async_result_rx = Some(rx);

                    let token = app.token.clone();
                    thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let result = rt.block_on(async { slack::fetch_channels(&token).await });
                        let _ = tx.send(AsyncResult::ChannelsLoaded(
                            result.map_err(|e| e.to_string()),
                        ));
                    });
                }
                KeyCode::Char('a') if *active_field == ConvExportField::Channels => {
                    if let Some(sel) = channel_selection {
                        sel.select_all();
                    }
                }
                KeyCode::Char('n') if *active_field == ConvExportField::Channels => {
                    if let Some(sel) = channel_selection {
                        sel.select_none();
                    }
                }
                KeyCode::Char(' ') if *active_field == ConvExportField::Channels => {
                    if let Some(sel) = channel_selection {
                        sel.toggle_current();
                    }
                }
                KeyCode::Up | KeyCode::Char('k')
                    if *active_field == ConvExportField::Channels =>
                {
                    if let Some(sel) = channel_selection {
                        sel.previous();
                    }
                }
                KeyCode::Down | KeyCode::Char('j')
                    if *active_field == ConvExportField::Channels =>
                {
                    if let Some(sel) = channel_selection {
                        sel.next();
                    }
                }
                KeyCode::Enter => {
                    let selected_channels = channel_selection
                        .as_ref()
                        .map(|s| s.selected.clone())
                        .unwrap_or_default();

                    if selected_channels.is_empty() {
                        return;
                    }

                    let selected_ids = channel_selection
                        .as_ref()
                        .map(|s| s.selected_ids())
                        .unwrap_or_default();

                    let from_date_str = from_date.text().to_string();
                    let to_date_str = to_date.text().to_string();
                    let output_path_str = output_path.text().to_string();

                    app.save_selected_channels(selected_ids);

                    let task = ExportTask::Conversations {
                        from_date: from_date_str.clone(),
                        to_date: to_date_str.clone(),
                        output_path: output_path_str,
                        selected_channels,
                    };
                    app.screen = Screen::Loading {
                        progress: None,
                        message: format!(
                            "Exporting conversations from {} to {}...",
                            from_date_str, to_date_str
                        ),
                    };
                    app.start_task(task);
                }
                _ if *active_field != ConvExportField::Channels => {
                    let field = match active_field {
                        ConvExportField::FromDate => from_date,
                        ConvExportField::ToDate => to_date,
                        ConvExportField::OutputPath => output_path,
                        ConvExportField::Channels => return,
                    };
                    field.handle_key(key);
                }
                _ => {}
            }
        }
        Screen::ExportUsers { output_path } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Char(c) => output_path.push(c),
            KeyCode::Backspace => {
                output_path.pop();
            }
            KeyCode::Enter => {
                let task = ExportTask::Users {
                    output_path: output_path.clone(),
                };
                app.screen = Screen::Loading {
                    message: "Exporting users...".to_string(),
                    progress: None,
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::ExportChannels { output_path } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Char(c) => output_path.push(c),
            KeyCode::Backspace => {
                output_path.pop();
            }
            KeyCode::Enter => {
                let task = ExportTask::Channels {
                    output_path: output_path.clone(),
                };
                app.screen = Screen::Loading {
                    message: "Exporting channels...".to_string(),
                    progress: None,
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::DownloadAttachments {
            conversations_path,
            output_path,
            active_field,
        } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Tab => {
                *active_field = match active_field {
                    DownloadAttachmentsField::ConversationsPath => {
                        DownloadAttachmentsField::OutputPath
                    }
                    DownloadAttachmentsField::OutputPath => {
                        DownloadAttachmentsField::ConversationsPath
                    }
                };
            }
            KeyCode::BackTab => {
                *active_field = match active_field {
                    DownloadAttachmentsField::ConversationsPath => {
                        DownloadAttachmentsField::OutputPath
                    }
                    DownloadAttachmentsField::OutputPath => {
                        DownloadAttachmentsField::ConversationsPath
                    }
                };
            }
            KeyCode::Char(c) => {
                let field = match active_field {
                    DownloadAttachmentsField::ConversationsPath => conversations_path,
                    DownloadAttachmentsField::OutputPath => output_path,
                };
                field.push(c);
            }
            KeyCode::Backspace => {
                let field = match active_field {
                    DownloadAttachmentsField::ConversationsPath => conversations_path,
                    DownloadAttachmentsField::OutputPath => output_path,
                };
                field.pop();
            }
            KeyCode::Enter => {
                let task = ExportTask::DownloadAttachments {
                    conversations_path: conversations_path.clone(),
                    output_path: output_path.clone(),
                };
                app.screen = Screen::Loading {
                    message: "Downloading attachments...".to_string(),
                    progress: None,
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::MarkdownExport {
            conversations_path,
            users_path,
            channels_path,
            output_path,
            active_field,
        } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Tab => {
                *active_field = match active_field {
                    MarkdownExportField::Conversations => MarkdownExportField::Users,
                    MarkdownExportField::Users => MarkdownExportField::Channels,
                    MarkdownExportField::Channels => MarkdownExportField::Output,
                    MarkdownExportField::Output => MarkdownExportField::Conversations,
                };
            }
            KeyCode::BackTab => {
                *active_field = match active_field {
                    MarkdownExportField::Conversations => MarkdownExportField::Output,
                    MarkdownExportField::Users => MarkdownExportField::Conversations,
                    MarkdownExportField::Channels => MarkdownExportField::Users,
                    MarkdownExportField::Output => MarkdownExportField::Channels,
                };
            }
            KeyCode::Char(c) => {
                let field = match active_field {
                    MarkdownExportField::Conversations => conversations_path,
                    MarkdownExportField::Users => users_path,
                    MarkdownExportField::Channels => channels_path,
                    MarkdownExportField::Output => output_path,
                };
                field.push(c);
            }
            KeyCode::Backspace => {
                let field = match active_field {
                    MarkdownExportField::Conversations => conversations_path,
                    MarkdownExportField::Users => users_path,
                    MarkdownExportField::Channels => channels_path,
                    MarkdownExportField::Output => output_path,
                };
                field.pop();
            }
            KeyCode::Enter => {
                let task = ExportTask::MarkdownExport {
                    conversations_path: conversations_path.clone(),
                    users_path: users_path.clone(),
                    channels_path: channels_path.clone(),
                    output_path: output_path.clone(),
                };
                app.screen = Screen::Loading {
                    message: "Exporting to markdown...".to_string(),
                    progress: None,
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::ExportEmojis {
            output_path,
            emojis_folder,
            active_field,
        } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Tab => {
                *active_field = match active_field {
                    ExportEmojisField::OutputPath => ExportEmojisField::EmojisFolder,
                    ExportEmojisField::EmojisFolder => ExportEmojisField::OutputPath,
                };
            }
            KeyCode::BackTab => {
                *active_field = match active_field {
                    ExportEmojisField::OutputPath => ExportEmojisField::EmojisFolder,
                    ExportEmojisField::EmojisFolder => ExportEmojisField::OutputPath,
                };
            }
            KeyCode::Char(c) => {
                let field = match active_field {
                    ExportEmojisField::OutputPath => output_path,
                    ExportEmojisField::EmojisFolder => emojis_folder,
                };
                field.push(c);
            }
            KeyCode::Backspace => {
                let field = match active_field {
                    ExportEmojisField::OutputPath => output_path,
                    ExportEmojisField::EmojisFolder => emojis_folder,
                };
                field.pop();
            }
            KeyCode::Enter => {
                let task = ExportTask::ExportEmojis {
                    output_path: output_path.clone(),
                    emojis_folder: emojis_folder.clone(),
                };
                app.screen = Screen::Loading {
                    message: "Exporting custom emojis...".to_string(),
                    progress: None,
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::ExportIndex {
            conversations_path,
            users_path,
            channels_path,
            output_path,
            active_field,
        } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Tab => {
                *active_field = match active_field {
                    ExportIndexField::Conversations => ExportIndexField::Users,
                    ExportIndexField::Users => ExportIndexField::Channels,
                    ExportIndexField::Channels => ExportIndexField::Output,
                    ExportIndexField::Output => ExportIndexField::Conversations,
                };
            }
            KeyCode::BackTab => {
                *active_field = match active_field {
                    ExportIndexField::Conversations => ExportIndexField::Output,
                    ExportIndexField::Users => ExportIndexField::Conversations,
                    ExportIndexField::Channels => ExportIndexField::Users,
                    ExportIndexField::Output => ExportIndexField::Channels,
                };
            }
            KeyCode::Char(c) => {
                let field = match active_field {
                    ExportIndexField::Conversations => conversations_path,
                    ExportIndexField::Users => users_path,
                    ExportIndexField::Channels => channels_path,
                    ExportIndexField::Output => output_path,
                };
                field.push(c);
            }
            KeyCode::Backspace => {
                let field = match active_field {
                    ExportIndexField::Conversations => conversations_path,
                    ExportIndexField::Users => users_path,
                    ExportIndexField::Channels => channels_path,
                    ExportIndexField::Output => output_path,
                };
                field.pop();
            }
            KeyCode::Enter => {
                let task = ExportTask::ExportIndex {
                    conversations_path: conversations_path.clone(),
                    users_path: users_path.clone(),
                    channels_path: channels_path.clone(),
                    output_path: output_path.clone(),
                };
                app.screen = Screen::Loading {
                    message: "Exporting to index...".to_string(),
                    progress: None,
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::ImportMeilisearch {
            input_path,
            url,
            api_key,
            index_name,
            clear,
            active_field,
        } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Tab => {
                *active_field = match active_field {
                    ImportMeilisearchField::Input => ImportMeilisearchField::Url,
                    ImportMeilisearchField::Url => ImportMeilisearchField::ApiKey,
                    ImportMeilisearchField::ApiKey => ImportMeilisearchField::IndexName,
                    ImportMeilisearchField::IndexName => ImportMeilisearchField::Clear,
                    ImportMeilisearchField::Clear => ImportMeilisearchField::Input,
                };
            }
            KeyCode::BackTab => {
                *active_field = match active_field {
                    ImportMeilisearchField::Input => ImportMeilisearchField::Clear,
                    ImportMeilisearchField::Url => ImportMeilisearchField::Input,
                    ImportMeilisearchField::ApiKey => ImportMeilisearchField::Url,
                    ImportMeilisearchField::IndexName => ImportMeilisearchField::ApiKey,
                    ImportMeilisearchField::Clear => ImportMeilisearchField::IndexName,
                };
            }
            KeyCode::Char(' ') if *active_field == ImportMeilisearchField::Clear => {
                *clear = !*clear;
            }
            KeyCode::Char(c) if *active_field != ImportMeilisearchField::Clear => {
                let field = match active_field {
                    ImportMeilisearchField::Input => input_path,
                    ImportMeilisearchField::Url => url,
                    ImportMeilisearchField::ApiKey => api_key,
                    ImportMeilisearchField::IndexName => index_name,
                    ImportMeilisearchField::Clear => return,
                };
                field.push(c);
            }
            KeyCode::Backspace if *active_field != ImportMeilisearchField::Clear => {
                let field = match active_field {
                    ImportMeilisearchField::Input => input_path,
                    ImportMeilisearchField::Url => url,
                    ImportMeilisearchField::ApiKey => api_key,
                    ImportMeilisearchField::IndexName => index_name,
                    ImportMeilisearchField::Clear => return,
                };
                field.pop();
            }
            KeyCode::Enter => {
                let task = ExportTask::ImportMeilisearch {
                    input_path: input_path.clone(),
                    url: url.clone(),
                    api_key: api_key.clone(),
                    index_name: index_name.clone(),
                    clear: *clear,
                };
                let msg = if *clear {
                    "Importing to Meilisearch (clearing index)...".to_string()
                } else {
                    "Importing to Meilisearch...".to_string()
                };
                app.screen = Screen::Loading {
                    message: msg,
                    progress: None,
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::QueryMeilisearch {
            query,
            url,
            api_key,
            index_name,
            active_field,
            results,
            result_state,
            error,
        } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Tab => {
                *active_field = match active_field {
                    QueryMeilisearchField::Query => QueryMeilisearchField::Url,
                    QueryMeilisearchField::Url => QueryMeilisearchField::ApiKey,
                    QueryMeilisearchField::ApiKey => QueryMeilisearchField::IndexName,
                    QueryMeilisearchField::IndexName => QueryMeilisearchField::Query,
                };
            }
            KeyCode::BackTab => {
                *active_field = match active_field {
                    QueryMeilisearchField::Query => QueryMeilisearchField::IndexName,
                    QueryMeilisearchField::Url => QueryMeilisearchField::Query,
                    QueryMeilisearchField::ApiKey => QueryMeilisearchField::Url,
                    QueryMeilisearchField::IndexName => QueryMeilisearchField::ApiKey,
                };
            }
            KeyCode::Up | KeyCode::Char('k') if results.is_some() => {
                if let Some(res) = results
                    && !res.is_empty()
                {
                    let i = match result_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                res.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    result_state.select(Some(i));
                }
            }
            KeyCode::Down | KeyCode::Char('j') if results.is_some() => {
                if let Some(res) = results
                    && !res.is_empty()
                {
                    let i = match result_state.selected() {
                        Some(i) => {
                            if i >= res.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    result_state.select(Some(i));
                }
            }
            KeyCode::Char(c) => {
                let field = match active_field {
                    QueryMeilisearchField::Query => query,
                    QueryMeilisearchField::Url => url,
                    QueryMeilisearchField::ApiKey => api_key,
                    QueryMeilisearchField::IndexName => index_name,
                };
                field.push(c);
            }
            KeyCode::Backspace => {
                let field = match active_field {
                    QueryMeilisearchField::Query => query,
                    QueryMeilisearchField::Url => url,
                    QueryMeilisearchField::ApiKey => api_key,
                    QueryMeilisearchField::IndexName => index_name,
                };
                field.pop();
            }
            KeyCode::Enter => {
                // Start async query
                *error = None;
                let (tx, rx) = mpsc::channel();
                app.async_result_rx = Some(rx);

                let query_str = query.clone();
                let url_str = url.clone();
                let api_key_str = api_key.clone();
                let index_name_str = index_name.clone();

                thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let result = rt.block_on(async {
                        crate::meilisearch::query_meilisearch(
                            &url_str,
                            &api_key_str,
                            &index_name_str,
                            &query_str,
                            20,
                        )
                        .await
                    });
                    let _ = tx.send(AsyncResult::QueryResult(
                        result.map(|r| r.hits).map_err(|e| e.to_string()),
                    ));
                });
            }
            _ => {}
        },
        Screen::EditConversationsPathInput {
            conversations_path,
            users_path,
            channels_path,
            active_field,
        } => match key.code {
            KeyCode::Esc => app.screen = Screen::MainMenu,
            KeyCode::Tab => {
                *active_field = match active_field {
                    EditConvPathField::Conversations => EditConvPathField::Users,
                    EditConvPathField::Users => EditConvPathField::Channels,
                    EditConvPathField::Channels => EditConvPathField::Conversations,
                };
            }
            KeyCode::BackTab => {
                *active_field = match active_field {
                    EditConvPathField::Conversations => EditConvPathField::Channels,
                    EditConvPathField::Users => EditConvPathField::Conversations,
                    EditConvPathField::Channels => EditConvPathField::Users,
                };
            }
            KeyCode::Char(c) => {
                let field = match active_field {
                    EditConvPathField::Conversations => conversations_path,
                    EditConvPathField::Users => users_path,
                    EditConvPathField::Channels => channels_path,
                };
                field.push(c);
            }
            KeyCode::Backspace => {
                let field = match active_field {
                    EditConvPathField::Conversations => conversations_path,
                    EditConvPathField::Users => users_path,
                    EditConvPathField::Channels => channels_path,
                };
                field.pop();
            }
            KeyCode::Enter => {
                let conv_path = conversations_path.clone();
                let usr_path = users_path.clone();
                let ch_path = channels_path.clone();

                match slack::load_conversations_for_editing(&conv_path, &usr_path, &ch_path) {
                    Ok((channels, users, channel_data)) => {
                        app.screen = Screen::EditConversationsChannelList {
                            channels: EditableChannelList::new(channels),
                            users,
                            channel_data,
                            editing_export_path: false,
                        };
                    }
                    Err(e) => {
                        app.screen = Screen::Error {
                            message: e.to_string(),
                        };
                    }
                }
            }
            _ => {}
        },
        Screen::EditConversationsChannelList {
            channels,
            users,
            channel_data,
            editing_export_path,
        } => {
            if *editing_export_path {
                match key.code {
                    KeyCode::Esc => *editing_export_path = false,
                    KeyCode::Char(c) => channels.export_path.push(c),
                    KeyCode::Backspace => {
                        channels.export_path.pop();
                    }
                    KeyCode::Enter => {
                        *editing_export_path = false;
                    }
                    _ => {}
                }
            } else {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => app.screen = Screen::MainMenu,
                    (KeyCode::Up, KeyModifiers::ALT) => channels.move_current_up(),
                    (KeyCode::Down, KeyModifiers::ALT) => channels.move_current_down(),
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => channels.previous(),
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => channels.next(),
                    (KeyCode::Enter, _) => {
                        if let Some(idx) = channels.list_state.selected() {
                            app.screen = Screen::EditConversationsMessageList {
                                channel_idx: idx,
                                channels: channels.clone(),
                                users: users.clone(),
                                channel_data: channel_data.clone(),
                            };
                        }
                    }
                    (KeyCode::Char('e'), _) => {
                        let export_path = channels.export_path.clone();
                        let export_data = channels.to_export_data();
                        match slack::export_edited_conversations_to_file(&export_data, &export_path)
                        {
                            Ok(count) => {
                                app.screen = Screen::Success {
                                    message: format!(
                                        "Exported {} messages to {}",
                                        count, export_path
                                    ),
                                };
                            }
                            Err(e) => {
                                app.screen = Screen::Error {
                                    message: e.to_string(),
                                };
                            }
                        }
                    }
                    (KeyCode::Char('p'), _) => {
                        *editing_export_path = true;
                    }
                    _ => {}
                }
            }
        }
        Screen::EditConversationsMessageList {
            channel_idx,
            channels,
            users,
            channel_data,
        } => {
            if let Some(channel) = channels.channels.get_mut(*channel_idx) {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => {
                        app.screen = Screen::EditConversationsChannelList {
                            channels: channels.clone(),
                            users: users.clone(),
                            channel_data: channel_data.clone(),
                            editing_export_path: false,
                        };
                    }
                    (KeyCode::Up, KeyModifiers::ALT) => {
                        channel.move_current_up();
                    }
                    (KeyCode::Down, KeyModifiers::ALT) => {
                        channel.move_current_down();
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                        channel.previous();
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        channel.next();
                    }
                    (KeyCode::Char(' '), _) => {
                        channel.toggle_current();
                    }
                    (KeyCode::Tab, _) => {
                        channel.toggle_collapse_current();
                    }
                    (KeyCode::Enter, _) => {
                        if let Some(msg_idx) = channel.list_state.selected() {
                            let mut attachment_list_state = ListState::default();
                            let msg = &channel.messages[msg_idx];
                            if !msg.files().is_empty() || !msg.links().is_empty() {
                                attachment_list_state.select(Some(0));
                            }
                            app.screen = Screen::EditConversationsMessageDetail {
                                channel_idx: *channel_idx,
                                message_idx: msg_idx,
                                channels: channels.clone(),
                                users: users.clone(),
                                channel_data: channel_data.clone(),
                                attachment_list_state,
                                editing_title: None,
                            };
                        }
                    }
                    (KeyCode::Char('a'), _) => {
                        for msg in &mut channel.messages {
                            msg.enabled = true;
                        }
                    }
                    (KeyCode::Char('n'), _) => {
                        for msg in &mut channel.messages {
                            msg.enabled = false;
                        }
                    }
                    _ => {}
                }
            }
        }
        Screen::EditConversationsMessageDetail {
            channel_idx,
            message_idx,
            channels,
            users,
            channel_data,
            attachment_list_state,
            editing_title,
        } => {
            // Handle title editing mode
            if let Some((link_idx, text_input)) = editing_title {
                match key.code {
                    KeyCode::Esc => {
                        *editing_title = None;
                    }
                    KeyCode::Enter => {
                        if let Some(channel) = channels.channels.get_mut(*channel_idx)
                            && let Some(msg) = channel.messages.get_mut(*message_idx)
                        {
                            let files_count = msg.files().len();
                            if *link_idx < files_count {
                                msg.set_file_title(*link_idx, text_input.text().to_string());
                            } else {
                                let lp_idx = *link_idx - files_count;
                                msg.set_link_title(lp_idx, text_input.text().to_string());
                            }
                        }
                        *editing_title = None;
                    }
                    _ => {
                        text_input.handle_key(key);
                    }
                }
                return;
            }

            if let Some(channel) = channels.channels.get_mut(*channel_idx)
                && let Some(msg) = channel.messages.get_mut(*message_idx)
            {
                let files_count = msg.files().len();
                let links_count = msg.links().len();
                let total_items = files_count + links_count;

                match key.code {
                    KeyCode::Esc => {
                        app.screen = Screen::EditConversationsMessageList {
                            channel_idx: *channel_idx,
                            channels: channels.clone(),
                            users: users.clone(),
                            channel_data: channel_data.clone(),
                        };
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if total_items > 0 {
                            let i = match attachment_list_state.selected() {
                                Some(i) => {
                                    if i == 0 {
                                        total_items - 1
                                    } else {
                                        i - 1
                                    }
                                }
                                None => 0,
                            };
                            attachment_list_state.select(Some(i));
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if total_items > 0 {
                            let i = match attachment_list_state.selected() {
                                Some(i) => {
                                    if i >= total_items - 1 {
                                        0
                                    } else {
                                        i + 1
                                    }
                                }
                                None => 0,
                            };
                            attachment_list_state.select(Some(i));
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(idx) = attachment_list_state.selected() {
                            if idx < files_count {
                                if msg.selected_files.contains(&idx) {
                                    msg.selected_files.remove(&idx);
                                } else {
                                    msg.selected_files.insert(idx);
                                }
                            } else {
                                let lp_idx = idx - files_count;
                                if msg.selected_link_previews.contains(&lp_idx) {
                                    msg.selected_link_previews.remove(&lp_idx);
                                } else {
                                    msg.selected_link_previews.insert(lp_idx);
                                }
                            }
                        }
                    }
                    KeyCode::Char('m') => {
                        if let Some(idx) = attachment_list_state.selected()
                            && idx >= files_count
                        {
                            let lp_idx = idx - files_count;
                            msg.main_link = Some(lp_idx);
                        }
                    }
                    KeyCode::Char('e') => {
                        if let Some(idx) = attachment_list_state.selected() {
                            let current_title = if idx < files_count {
                                msg.get_file_title(idx).unwrap_or_default()
                            } else {
                                let lp_idx = idx - files_count;
                                msg.get_link_title(lp_idx).unwrap_or_default()
                            };
                            *editing_title = Some((idx, TextInput::new(current_title)));
                        }
                    }
                    KeyCode::Char('f') => {
                        if let Some(idx) = attachment_list_state.selected()
                            && idx >= files_count
                        {
                            let lp_idx = idx - files_count;
                            if let Some(url) = msg.get_link_url(lp_idx) {
                                match webpage::Webpage::from_url(
                                    &url,
                                    webpage::WebpageOptions::default(),
                                ) {
                                    Ok(page) => {
                                        if let Some(title) = page.html.title {
                                            msg.set_link_title(lp_idx, title);
                                        }
                                    }
                                    Err(_) => {
                                        // Silently ignore fetch errors
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('a') => {
                        msg.selected_files = (0..files_count).collect();
                        msg.selected_link_previews = (0..links_count).collect();
                    }
                    KeyCode::Char('n') => {
                        msg.selected_files.clear();
                        msg.selected_link_previews.clear();
                    }
                    _ => {}
                }
            }
        }
        Screen::Loading { .. } => {}
        Screen::Success { .. } | Screen::Error { .. } => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.screen = Screen::MainMenu;
                app.menu_state.select(Some(0));
            }
            _ => {}
        },
    }
}
