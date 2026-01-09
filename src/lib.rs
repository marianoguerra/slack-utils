use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use thiserror::Error;

mod settings;
mod slack;

use settings::Settings;
use slack::ChannelInfo;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("SLACK_TOKEN environment variable not set")]
    MissingToken,

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid date format: {0}")]
    InvalidDate(String),

    #[error("Slack API error: {0}")]
    SlackApi(String),

    #[error("failed to read file at {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to write file at {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("JSON serialization error: {0}")]
    JsonSerialize(String),

    #[error("JSON parse error: {0}")]
    JsonParse(String),

    #[error("TOML parse error: {0}")]
    TomlParse(String),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

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
}

pub fn load_token() -> Result<String> {
    std::env::var("SLACK_TOKEN").map_err(|_| AppError::MissingToken)
}

pub fn default_from_date() -> NaiveDate {
    Local::now().date_naive() - chrono::Duration::days(7)
}

pub fn default_to_date() -> NaiveDate {
    Local::now().date_naive()
}

pub fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| AppError::InvalidDate(s.to_string()))
}

const CHANNELS_FILE: &str = "channels.json";

#[derive(Debug, Clone, Copy, PartialEq)]
enum MenuItem {
    ExportConversations,
    ExportUsers,
    ExportChannels,
    Exit,
}

impl MenuItem {
    fn all() -> Vec<MenuItem> {
        vec![
            MenuItem::ExportConversations,
            MenuItem::ExportUsers,
            MenuItem::ExportChannels,
            MenuItem::Exit,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            MenuItem::ExportConversations => "Export Conversations in Date Range",
            MenuItem::ExportUsers => "Export Users",
            MenuItem::ExportChannels => "Export Channels",
            MenuItem::Exit => "Exit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConvExportField {
    FromDate,
    ToDate,
    OutputPath,
    Channels,
}

#[derive(Debug, Clone)]
struct ChannelSelection {
    channels: Vec<ChannelInfo>,
    selected: HashSet<String>,
    list_state: ListState,
}

impl ChannelSelection {
    fn new(channels: Vec<ChannelInfo>, saved_selection: Option<HashSet<String>>) -> Self {
        let mut channels = channels;
        channels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let selected = match saved_selection {
            Some(saved) if !saved.is_empty() => {
                let channel_ids: HashSet<_> = channels.iter().map(|c| c.id.clone()).collect();
                saved.into_iter().filter(|id| channel_ids.contains(id)).collect()
            }
            _ => channels.iter().map(|c| c.id.clone()).collect(),
        };

        let mut list_state = ListState::default();
        if !channels.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            channels,
            selected,
            list_state,
        }
    }

    fn toggle_current(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(channel) = self.channels.get(idx)
        {
            if self.selected.contains(&channel.id) {
                self.selected.remove(&channel.id);
            } else {
                self.selected.insert(channel.id.clone());
            }
        }
    }

    fn select_all(&mut self) {
        self.selected = self.channels.iter().map(|c| c.id.clone()).collect();
    }

    fn select_none(&mut self) {
        self.selected.clear();
    }

    fn next(&mut self) {
        if self.channels.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.channels.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.channels.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.channels.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn selected_ids(&self) -> Vec<String> {
        self.selected.iter().cloned().collect()
    }
}

#[derive(Debug, Clone)]
enum ExportTask {
    Conversations {
        from_date: String,
        to_date: String,
        output_path: String,
        selected_channels: HashSet<String>,
    },
    Users {
        output_path: String,
    },
    Channels {
        output_path: String,
    },
}

#[derive(Debug, Clone)]
enum Screen {
    MainMenu,
    ExportConversations {
        from_date: String,
        to_date: String,
        output_path: String,
        active_field: ConvExportField,
        channel_selection: Option<ChannelSelection>,
        loading_channels: bool,
    },
    ExportUsers {
        output_path: String,
    },
    ExportChannels {
        output_path: String,
    },
    Loading {
        message: String,
    },
    Success {
        message: String,
    },
    Error {
        message: String,
    },
}

enum AsyncResult {
    ExportComplete(std::result::Result<String, String>),
    ChannelsLoaded(std::result::Result<Vec<ChannelInfo>, String>),
}

struct App {
    screen: Screen,
    menu_state: ListState,
    should_quit: bool,
    token: String,
    async_result_rx: Option<mpsc::Receiver<AsyncResult>>,
    settings: Settings,
}

impl App {
    fn new(token: String) -> Self {
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));

        let settings = Settings::load().unwrap_or_default();

        Self {
            screen: Screen::MainMenu,
            menu_state,
            should_quit: false,
            token,
            async_result_rx: None,
            settings,
        }
    }

    fn menu_next(&mut self) {
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

    fn menu_previous(&mut self) {
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

    fn selected_menu_item(&self) -> MenuItem {
        let idx = self.menu_state.selected().unwrap_or(0);
        MenuItem::all()[idx]
    }

    fn start_task(&mut self, task: ExportTask) {
        let (tx, rx) = mpsc::channel();
        self.async_result_rx = Some(rx);

        let token = self.token.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match task {
                ExportTask::Users { output_path } => {
                    let result = rt.block_on(async {
                        let count = slack::export_users(&token, Path::new(&output_path)).await?;
                        Ok::<_, AppError>(format!("Exported {} users to {}", count, output_path))
                    });
                    let _ = tx.send(AsyncResult::ExportComplete(
                        result.map_err(|e| e.to_string()),
                    ));
                }
                ExportTask::Channels { output_path } => {
                    let result = rt.block_on(async {
                        let count = slack::export_channels(&token, Path::new(&output_path)).await?;
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
                } => {
                    let result = rt.block_on(async {
                        let from = parse_date(&from_date)?;
                        let to = parse_date(&to_date)?;
                        let count = slack::export_conversations(
                            &token,
                            from,
                            to,
                            Path::new(&output_path),
                            Some(&selected_channels),
                        )
                        .await?;
                        Ok::<_, AppError>(format!("Exported {} messages to {}", count, output_path))
                    });
                    let _ = tx.send(AsyncResult::ExportComplete(
                        result.map_err(|e| e.to_string()),
                    ));
                }
            }
        });
    }

    fn check_async_result(&mut self) {
        if let Some(rx) = &self.async_result_rx
            && let Ok(result) = rx.try_recv()
        {
            self.async_result_rx = None;
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
            }
        }
    }

    fn open_export_conversations(&mut self) {
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
            from_date: default_from_date().format("%Y-%m-%d").to_string(),
            to_date: default_to_date().format("%Y-%m-%d").to_string(),
            output_path: "./conversations.json".to_string(),
            active_field: ConvExportField::FromDate,
            channel_selection,
            loading_channels,
        };
    }

    fn save_selected_channels(&mut self, channels: Vec<String>) {
        self.settings.set_selected_channels(channels);
        let _ = self.settings.save();
    }
}

pub fn run_ui() -> Result<()> {
    let token = load_token()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(token);
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        app.check_async_result();

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            handle_input(app, key.code);
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_input(app: &mut App, key: KeyCode) {
    match &mut app.screen {
        Screen::MainMenu => match key {
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

            match key {
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
                KeyCode::Char(c) if *active_field != ConvExportField::Channels => {
                    let field = match active_field {
                        ConvExportField::FromDate => from_date,
                        ConvExportField::ToDate => to_date,
                        ConvExportField::OutputPath => output_path,
                        ConvExportField::Channels => return,
                    };
                    field.push(c);
                }
                KeyCode::Backspace if *active_field != ConvExportField::Channels => {
                    let field = match active_field {
                        ConvExportField::FromDate => from_date,
                        ConvExportField::ToDate => to_date,
                        ConvExportField::OutputPath => output_path,
                        ConvExportField::Channels => return,
                    };
                    field.pop();
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

                    let from_date_clone = from_date.clone();
                    let to_date_clone = to_date.clone();
                    let output_path_clone = output_path.clone();

                    app.save_selected_channels(selected_ids);

                    let task = ExportTask::Conversations {
                        from_date: from_date_clone.clone(),
                        to_date: to_date_clone.clone(),
                        output_path: output_path_clone,
                        selected_channels,
                    };
                    app.screen = Screen::Loading {
                        message: format!(
                            "Exporting conversations from {} to {}...",
                            from_date_clone, to_date_clone
                        ),
                    };
                    app.start_task(task);
                }
                _ => {}
            }
        }
        Screen::ExportUsers { output_path } => match key {
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
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::ExportChannels { output_path } => match key {
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
                };
                app.start_task(task);
            }
            _ => {}
        },
        Screen::Loading { .. } => {}
        Screen::Success { .. } | Screen::Error { .. } => match key {
            KeyCode::Enter | KeyCode::Esc => {
                app.screen = Screen::MainMenu;
                app.menu_state.select(Some(0));
            }
            _ => {}
        },
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    let banner = Paragraph::new("Slack Utils")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(banner, chunks[0]);

    match &mut app.screen {
        Screen::MainMenu => render_main_menu(f, app, chunks[1]),
        Screen::ExportConversations {
            from_date,
            to_date,
            output_path,
            active_field,
            channel_selection,
            loading_channels,
        } => render_export_conversations(
            f,
            from_date,
            to_date,
            output_path,
            *active_field,
            channel_selection.as_mut(),
            *loading_channels,
            chunks[1],
        ),
        Screen::ExportUsers { output_path } => {
            render_export_simple(f, "Export Users", output_path, chunks[1])
        }
        Screen::ExportChannels { output_path } => {
            render_export_simple(f, "Export Channels", output_path, chunks[1])
        }
        Screen::Loading { message, .. } => render_loading(f, message, chunks[1]),
        Screen::Success { message } => render_success(f, message, chunks[1]),
        Screen::Error { message } => render_error(f, message, chunks[1]),
    }
}

fn render_main_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = MenuItem::all()
        .iter()
        .map(|item| ListItem::new(Line::from(Span::raw(item.label()))))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Main Menu"))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.menu_state);

    let help = Paragraph::new("↑/↓: Navigate | Enter: Select | q: Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    let help_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    f.render_widget(help, help_area);
}

#[allow(clippy::too_many_arguments)]
fn render_export_conversations(
    f: &mut Frame,
    from_date: &str,
    to_date: &str,
    output_path: &str,
    active_field: ConvExportField,
    channel_selection: Option<&mut ChannelSelection>,
    loading_channels: bool,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Export Conversations");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(inner);

    let from_style = if active_field == ConvExportField::FromDate {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let from_input = Paragraph::new(from_date).style(from_style).block(
        Block::default()
            .borders(Borders::ALL)
            .title("From Date (YYYY-MM-DD)"),
    );
    f.render_widget(from_input, chunks[0]);

    let to_style = if active_field == ConvExportField::ToDate {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let to_input = Paragraph::new(to_date).style(to_style).block(
        Block::default()
            .borders(Borders::ALL)
            .title("To Date (YYYY-MM-DD)"),
    );
    f.render_widget(to_input, chunks[1]);

    let output_style = if active_field == ConvExportField::OutputPath {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let output_input = Paragraph::new(output_path)
        .style(output_style)
        .block(Block::default().borders(Borders::ALL).title("Output Path"));
    f.render_widget(output_input, chunks[2]);

    let channels_block_style = if active_field == ConvExportField::Channels {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    if loading_channels {
        let loading = Paragraph::new("Loading channels...")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Channels")
                    .border_style(channels_block_style),
            );
        f.render_widget(loading, chunks[3]);
    } else if let Some(sel) = channel_selection {
        let selected_count = sel.selected.len();
        let total_count = sel.channels.len();

        let items: Vec<ListItem> = sel
            .channels
            .iter()
            .map(|ch| {
                let checkbox = if sel.selected.contains(&ch.id) {
                    "[x]"
                } else {
                    "[ ]"
                };
                ListItem::new(Line::from(format!("{} #{}", checkbox, ch.name)))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Channels ({}/{})", selected_count, total_count))
                    .border_style(channels_block_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, chunks[3], &mut sel.list_state);
    } else {
        let no_channels = Paragraph::new("No channels loaded. Press 'r' to fetch from Slack.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Channels")
                    .border_style(channels_block_style),
            );
        f.render_widget(no_channels, chunks[3]);
    }

    let help_text = if active_field == ConvExportField::Channels {
        "↑/↓: Navigate | Space: Toggle | a: All | n: None | r: Refresh | Tab: Next | Enter: Export | Esc: Back"
    } else {
        "Tab: Next Field | Enter: Export | Esc: Back"
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);
}

fn render_export_simple(f: &mut Frame, title: &str, output_path: &str, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(inner);

    let output_input = Paragraph::new(output_path)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Output Path"));
    f.render_widget(output_input, chunks[0]);

    let help = Paragraph::new("Enter: Confirm | Esc: Back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[1]);
}

fn render_loading(f: &mut Frame, message: &str, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Processing");

    let popup_area = centered_rect(60, 30, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(inner);

    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / 100) as usize
        % spinner_frames.len();

    let spinner = Paragraph::new(format!("{}  Loading...", spinner_frames[frame_idx]))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(spinner, chunks[0]);

    let msg = Paragraph::new(message)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(msg, chunks[1]);
}

fn render_success(f: &mut Frame, message: &str, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Success")
        .border_style(Style::default().fg(Color::Green));

    let popup_area = centered_rect(60, 30, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(inner);

    let checkmark = Paragraph::new("✓")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(checkmark, chunks[0]);

    let msg = Paragraph::new(message)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(msg, chunks[1]);

    let help = Paragraph::new("Press Enter to continue")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}

fn render_error(f: &mut Frame, message: &str, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Error")
        .border_style(Style::default().fg(Color::Red));

    let popup_area = centered_rect(70, 40, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(inner);

    let icon = Paragraph::new("✗")
        .style(
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(icon, chunks[0]);

    let msg = Paragraph::new(message)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(msg, chunks[1]);

    let help = Paragraph::new("Press Enter to continue")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub async fn run_export_conversations_async(
    from: Option<String>,
    to: Option<String>,
    output: &str,
) -> Result<()> {
    let token = load_token()?;

    let from_date = match from {
        Some(s) => parse_date(&s)?,
        None => default_from_date(),
    };
    let to_date = match to {
        Some(s) => parse_date(&s)?,
        None => default_to_date(),
    };

    println!(
        "Exporting conversations from {} to {} to {}...",
        from_date, to_date, output
    );

    let count =
        slack::export_conversations(&token, from_date, to_date, Path::new(output), None).await?;

    println!(
        "Export completed successfully! {} messages exported.",
        count
    );
    Ok(())
}

pub async fn run_export_users_async(output: &str) -> Result<()> {
    let token = load_token()?;

    println!("Exporting users to {}...", output);

    let count = slack::export_users(&token, Path::new(output)).await?;

    println!("Export completed successfully! {} users exported.", count);
    Ok(())
}

pub async fn run_export_channels_async(output: &str) -> Result<()> {
    let token = load_token()?;

    println!("Exporting channels to {}...", output);

    let count = slack::export_channels(&token, Path::new(output)).await?;

    println!(
        "Export completed successfully! {} channels exported.",
        count
    );
    Ok(())
}
