use std::path::Path;
use std::sync::mpsc;
use std::thread;

use ratatui::widgets::ListState;

use crate::error::AppError;
use crate::index::export_conversations_to_index_with_progress;
use crate::markdown::export_conversations_to_markdown_with_progress;
use crate::meilisearch::import_index_to_meilisearch;
use crate::settings::Settings;
use crate::slack;
use crate::ui::types::{
    AsyncResult, ChannelSelection, ConvExportField, ConvExportWeekField, ExportTask, MenuItem,
    Screen,
};
use crate::widgets::TextInput;
use crate::{
    current_iso_week, default_from_date, default_to_date, parse_date, week_to_date_range,
    CHANNELS_FILE,
};

pub struct App {
    pub screen: Screen,
    pub menu_state: ListState,
    pub should_quit: bool,
    pub token: String,
    pub async_result_rx: Option<mpsc::Receiver<AsyncResult>>,
    pub progress_rx: Option<mpsc::Receiver<(usize, usize, String)>>,
    pub settings: Settings,
}

impl App {
    pub fn new(token: String) -> Self {
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));

        let settings = Settings::load().unwrap_or_default();

        Self {
            screen: Screen::MainMenu,
            menu_state,
            should_quit: false,
            token,
            async_result_rx: None,
            progress_rx: None,
            settings,
        }
    }

    pub fn menu_next(&mut self) {
        let items = MenuItem::all();
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i >= items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    pub fn menu_previous(&mut self) {
        let items = MenuItem::all();
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i == 0 {
                    items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    pub fn selected_menu_item(&self) -> MenuItem {
        let idx = self.menu_state.selected().unwrap_or(0);
        MenuItem::all()[idx]
    }

    pub fn start_task(&mut self, task: ExportTask) {
        let (tx, rx) = mpsc::channel();
        self.async_result_rx = Some(rx);

        let token = self.token.clone();

        let (progress_tx, progress_rx) = mpsc::channel();
        self.progress_rx = Some(progress_rx);

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match task {
                ExportTask::Users { output_path, format } => {
                    let result = rt.block_on(async {
                        let count = slack::export_users(&token, Path::new(&output_path), format).await?;
                        Ok::<_, AppError>(format!("Exported {} users to {}", count, output_path))
                    });
                    let _ = tx.send(AsyncResult::ExportComplete(
                        result.map_err(|e| e.to_string()),
                    ));
                }
                ExportTask::Channels { output_path, format } => {
                    let result = rt.block_on(async {
                        let count = slack::export_channels(&token, Path::new(&output_path), format).await?;
                        Ok::<_, AppError>(format!("Exported {} channels to {}", count, output_path))
                    });
                    let _ = tx.send(AsyncResult::ExportComplete(
                        result.map_err(|e| e.to_string()),
                    ));
                }
                ExportTask::Conversations {
                    from_date,
                    to_date,
                    output_path,
                    selected_channels,
                    format,
                } => {
                    let progress_callback = |current: usize, total: usize, name: &str| {
                        let _ = progress_tx.send((current, total, name.to_string()));
                    };
                    let result = rt.block_on(async {
                        let from = parse_date(&from_date)?;
                        let to = parse_date(&to_date)?;
                        let count = slack::export_conversations(
                            &token,
                            from,
                            to,
                            Path::new(&output_path),
                            Some(&selected_channels),
                            Some(progress_callback),
                            format,
                        )
                        .await?;
                        Ok::<_, AppError>(format!("Exported {} messages to {}", count, output_path))
                    });
                    let _ = tx.send(AsyncResult::ExportComplete(
                        result.map_err(|e| e.to_string()),
                    ));
                }
                ExportTask::ConversationsWeek {
                    year,
                    week,
                    output_path,
                    selected_channels,
                    format,
                } => {
                    let progress_callback = |current: usize, total: usize, name: &str| {
                        let _ = progress_tx.send((current, total, name.to_string()));
                    };
                    let result = rt.block_on(async {
                        let (from, to) = week_to_date_range(year, week)?;
                        let count = slack::export_conversations(
                            &token,
                            from,
                            to,
                            Path::new(&output_path),
                            Some(&selected_channels),
                            Some(progress_callback),
                            format,
                        )
                        .await?;
                        Ok::<_, AppError>(format!(
                            "Exported {} messages for {}-W{:02} to {}",
                            count, year, week, output_path
                        ))
                    });
                    let _ = tx.send(AsyncResult::ExportComplete(
                        result.map_err(|e| e.to_string()),
                    ));
                }
                ExportTask::DownloadAttachments {
                    conversations_path,
                    output_path,
                } => {
                    let progress_callback = move |current: usize, total: usize, name: &str| {
                        let _ = progress_tx.send((current, total, name.to_string()));
                    };
                    let result = slack::download_attachments(
                        &token,
                        &conversations_path,
                        Path::new(&output_path),
                        Some(&progress_callback),
                    );
                    let msg = match result {
                        Ok(r) => Ok(format!(
                            "Downloaded {} files to {} ({} skipped, {} failed)",
                            r.downloaded, output_path, r.skipped, r.failed
                        )),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = tx.send(AsyncResult::ExportComplete(msg));
                }
                ExportTask::MarkdownExport {
                    conversations_path,
                    users_path,
                    channels_path,
                    output_path,
                } => {
                    let progress_callback = move |current: usize, total: usize, name: &str| {
                        let _ = progress_tx.send((current, total, name.to_string()));
                    };
                    let result = export_conversations_to_markdown_with_progress(
                        &conversations_path,
                        &users_path,
                        &channels_path,
                        &output_path,
                        Some(&progress_callback),
                    );
                    let msg = match result {
                        Ok(count) => Ok(format!(
                            "Exported {} messages to {}",
                            count, output_path
                        )),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = tx.send(AsyncResult::ExportComplete(msg));
                }
                ExportTask::ExportEmojis {
                    output_path,
                    emojis_folder,
                } => {
                    let progress_callback = |current: usize, total: usize, name: &str| {
                        let _ = progress_tx.send((current, total, name.to_string()));
                    };
                    let result = rt.block_on(async {
                        let r = slack::fetch_emojis(
                            &token,
                            Path::new(&output_path),
                            Path::new(&emojis_folder),
                            Some(&progress_callback),
                        )
                        .await?;
                        Ok::<_, AppError>(format!(
                            "Fetched {} emojis to {} ({} downloaded, {} skipped, {} failed)",
                            r.total, output_path, r.downloaded, r.skipped, r.failed
                        ))
                    });
                    let _ = tx.send(AsyncResult::ExportComplete(
                        result.map_err(|e| e.to_string()),
                    ));
                }
                ExportTask::ExportIndex {
                    conversations_path,
                    users_path,
                    channels_path,
                    output_path,
                } => {
                    let progress_callback = move |current: usize, total: usize, name: &str| {
                        let _ = progress_tx.send((current, total, name.to_string()));
                    };
                    let result = export_conversations_to_index_with_progress(
                        &conversations_path,
                        &users_path,
                        &channels_path,
                        &output_path,
                        Some(&progress_callback),
                    );
                    let msg = match result {
                        Ok(count) => Ok(format!(
                            "Exported {} messages to {}",
                            count, output_path
                        )),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = tx.send(AsyncResult::ExportComplete(msg));
                }
                ExportTask::ImportMeilisearch {
                    input_path,
                    url,
                    api_key,
                    index_name,
                    clear,
                } => {
                    let progress_callback = move |current: usize, total: usize, name: &str| {
                        let _ = progress_tx.send((current, total, name.to_string()));
                    };
                    let result = rt.block_on(async {
                        import_index_to_meilisearch(
                            &input_path,
                            &url,
                            &api_key,
                            &index_name,
                            clear,
                            Some(&progress_callback),
                        )
                        .await
                    });
                    let msg = match result {
                        Ok(r) => Ok(format!(
                            "Imported {} documents to Meilisearch index '{}'",
                            r.total, r.index_name
                        )),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = tx.send(AsyncResult::ExportComplete(msg));
                }
            }
        });
    }

    pub fn check_async_result(&mut self) {
        if let Some(rx) = &self.async_result_rx
            && let Ok(result) = rx.try_recv()
        {
            self.async_result_rx = None;
            self.progress_rx = None;
            match result {
                AsyncResult::ExportComplete(Ok(msg)) => {
                    self.screen = Screen::Success { message: msg };
                }
                AsyncResult::ExportComplete(Err(msg)) => {
                    self.screen = Screen::Error { message: msg };
                }
                AsyncResult::ChannelsLoaded(Ok(channels)) => {
                    if let Screen::ExportConversations {
                        channel_selection,
                        loading_channels,
                        ..
                    } = &mut self.screen
                    {
                        let saved_selection = Some(self.settings.selected_channels_set());
                        *channel_selection =
                            Some(ChannelSelection::new(channels, saved_selection));
                        *loading_channels = false;
                    }
                }
                AsyncResult::ChannelsLoaded(Err(msg)) => {
                    self.screen = Screen::Error { message: msg };
                }
                AsyncResult::QueryResult(Ok(hits)) => {
                    if let Screen::QueryMeilisearch {
                        results,
                        result_state,
                        error,
                        ..
                    } = &mut self.screen
                    {
                        *results = Some(hits);
                        *error = None;
                        if results.as_ref().map(|r| !r.is_empty()).unwrap_or(false) {
                            result_state.select(Some(0));
                        }
                    }
                }
                AsyncResult::QueryResult(Err(msg)) => {
                    if let Screen::QueryMeilisearch { error, results, .. } = &mut self.screen {
                        *error = Some(msg);
                        *results = None;
                    }
                }
            }
        }
    }

    pub fn check_progress(&mut self) {
        if let Some(rx) = &self.progress_rx {
            let mut latest = None;
            while let Ok(progress) = rx.try_recv() {
                latest = Some(progress);
            }
            if let Some(progress) = latest
                && let Screen::Loading {
                    progress: screen_progress,
                    ..
                } = &mut self.screen
            {
                *screen_progress = Some(progress);
            }
        }
    }

    pub fn open_export_conversations(&mut self) {
        let channels_result = slack::load_channels_from_file(Path::new(CHANNELS_FILE));

        let (channel_selection, loading_channels) = match channels_result {
            Ok(channels) => {
                let saved_selection = Some(self.settings.selected_channels_set());
                (
                    Some(ChannelSelection::new(channels, saved_selection)),
                    false,
                )
            }
            Err(_) => (None, false),
        };

        self.screen = Screen::ExportConversations {
            from_date: TextInput::new(default_from_date().format("%Y-%m-%d").to_string()),
            to_date: TextInput::new(default_to_date().format("%Y-%m-%d").to_string()),
            output_path: TextInput::new("./conversations.json".to_string()),
            active_field: ConvExportField::FromDate,
            channel_selection,
            loading_channels,
        };
    }

    pub fn open_export_conversations_week(&mut self) {
        let channels_result = slack::load_channels_from_file(Path::new(CHANNELS_FILE));

        let (channel_selection, loading_channels) = match channels_result {
            Ok(channels) => {
                let saved_selection = Some(self.settings.selected_channels_set());
                (
                    Some(ChannelSelection::new(channels, saved_selection)),
                    false,
                )
            }
            Err(_) => (None, false),
        };

        let (year, week) = current_iso_week();

        self.screen = Screen::ExportConversationsWeek {
            year: TextInput::new(year.to_string()),
            week: TextInput::new(week.to_string()),
            output_path: TextInput::new("./conversations.json".to_string()),
            active_field: ConvExportWeekField::Year,
            channel_selection,
            loading_channels,
        };
    }

    pub fn save_selected_channels(&mut self, channels: Vec<String>) {
        self.settings.set_selected_channels(channels);
        let _ = self.settings.save();
    }
}
