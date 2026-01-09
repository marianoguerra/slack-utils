use std::collections::HashSet;
use ratatui::widgets::ListState;

use crate::slack::ChannelInfo;
use crate::widgets::TextInput;

// Field enums for different screens
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConvExportField {
    FromDate,
    ToDate,
    OutputPath,
    Channels,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditConvPathField {
    Conversations,
    Users,
    Channels,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DownloadAttachmentsField {
    ConversationsPath,
    OutputPath,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkdownExportField {
    Conversations,
    Users,
    Channels,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportEmojisField {
    OutputPath,
    EmojisFolder,
}

// Menu item enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuItem {
    ExportConversations,
    ExportUsers,
    ExportChannels,
    EditConversations,
    DownloadAttachments,
    SelectedConversationsToMarkdown,
    ExportEmojis,
    Exit,
}

impl MenuItem {
    pub fn all() -> Vec<MenuItem> {
        vec![
            MenuItem::ExportUsers,
            MenuItem::ExportChannels,
            MenuItem::ExportConversations,
            MenuItem::DownloadAttachments,
            MenuItem::EditConversations,
            MenuItem::SelectedConversationsToMarkdown,
            MenuItem::ExportEmojis,
            MenuItem::Exit,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            MenuItem::ExportUsers => "Fetch Users",
            MenuItem::ExportChannels => "Fetch Channels",
            MenuItem::ExportConversations => "Fetch Conversations in Date Range",
            MenuItem::DownloadAttachments => "Download Attachments",
            MenuItem::EditConversations => "Edit Conversations",
            MenuItem::SelectedConversationsToMarkdown => "Export Conversations to Markdown",
            MenuItem::ExportEmojis => "Export Custom Emojis",
            MenuItem::Exit => "Exit",
        }
    }
}

// Link extraction types
#[derive(Debug, Clone)]
pub struct ExtractedLink {
    pub url: String,
    pub title: String,
    #[allow(dead_code)]
    pub source: LinkSource,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LinkSource {
    Attachment(usize),
    Block,
}

// Editable message type
#[derive(Debug, Clone)]
pub struct EditableMessage {
    pub original: serde_json::Value,
    pub enabled: bool,
    pub collapsed: bool,
    pub selected_files: HashSet<usize>,
    pub selected_link_previews: HashSet<usize>,
    pub main_link: Option<usize>,
    pub custom_link_titles: std::collections::HashMap<usize, String>,
    pub custom_file_titles: std::collections::HashMap<usize, String>,
}

impl EditableMessage {
    pub fn new(original: serde_json::Value) -> Self {
        let files_count = original
            .get("files")
            .and_then(|f| f.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        let links = Self::extract_links_static(&original);
        let links_count = links.len();
        let main_link = if links_count > 0 { Some(0) } else { None };

        Self {
            original,
            enabled: true,
            collapsed: true,
            selected_files: (0..files_count).collect(),
            selected_link_previews: (0..links_count).collect(),
            main_link,
            custom_link_titles: std::collections::HashMap::new(),
            custom_file_titles: std::collections::HashMap::new(),
        }
    }

    pub fn text(&self) -> &str {
        self.original
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
    }

    pub fn user_id(&self) -> Option<&str> {
        self.original.get("user").and_then(|u| u.as_str())
    }

    #[allow(dead_code)]
    pub fn ts(&self) -> &str {
        self.original
            .get("ts")
            .and_then(|t| t.as_str())
            .unwrap_or("")
    }

    pub fn files(&self) -> Vec<&serde_json::Value> {
        self.original
            .get("files")
            .and_then(|f| f.as_array())
            .map(|a| a.iter().collect())
            .unwrap_or_default()
    }

    pub fn extract_links_static(original: &serde_json::Value) -> Vec<ExtractedLink> {
        let mut links = Vec::new();

        if let Some(attachments) = original.get("attachments").and_then(|a| a.as_array()) {
            for (idx, att) in attachments.iter().enumerate() {
                let url = att
                    .get("original_url")
                    .or_else(|| att.get("from_url"))
                    .or_else(|| att.get("title_link"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let title = att
                    .get("title")
                    .or_else(|| att.get("fallback"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("Untitled");
                if !url.is_empty() {
                    links.push(ExtractedLink {
                        url: url.to_string(),
                        title: title.to_string(),
                        source: LinkSource::Attachment(idx),
                    });
                }
            }
        }

        if let Some(blocks) = original.get("blocks").and_then(|b| b.as_array()) {
            for block in blocks {
                if block.get("type").and_then(|t| t.as_str()) == Some("rich_text")
                    && let Some(elements) = block.get("elements").and_then(|e| e.as_array())
                {
                    Self::extract_links_from_elements(elements, &mut links);
                }
            }
        }

        let mut seen_urls = std::collections::HashSet::new();
        links.retain(|link| seen_urls.insert(link.url.clone()));

        links
    }

    fn extract_links_from_elements(elements: &[serde_json::Value], links: &mut Vec<ExtractedLink>) {
        for element in elements {
            let elem_type = element.get("type").and_then(|t| t.as_str());

            match elem_type {
                Some("link") => {
                    if let Some(url) = element.get("url").and_then(|u| u.as_str()) {
                        let title = element
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or(url);
                        links.push(ExtractedLink {
                            url: url.to_string(),
                            title: title.to_string(),
                            source: LinkSource::Block,
                        });
                    }
                }
                Some("rich_text_section")
                | Some("rich_text_list")
                | Some("rich_text_quote")
                | Some("rich_text_preformatted") => {
                    if let Some(nested) = element.get("elements").and_then(|e| e.as_array()) {
                        Self::extract_links_from_elements(nested, links);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn links(&self) -> Vec<ExtractedLink> {
        Self::extract_links_static(&self.original)
    }

    pub fn get_link_url(&self, idx: usize) -> Option<String> {
        self.links().get(idx).map(|l| l.url.clone())
    }

    pub fn get_link_title(&self, idx: usize) -> Option<String> {
        if let Some(custom) = self.custom_link_titles.get(&idx) {
            Some(custom.clone())
        } else {
            self.links().get(idx).map(|l| l.title.clone())
        }
    }

    pub fn set_link_title(&mut self, idx: usize, title: String) {
        self.custom_link_titles.insert(idx, title);
    }

    pub fn get_file_title(&self, idx: usize) -> Option<String> {
        if let Some(custom) = self.custom_file_titles.get(&idx) {
            Some(custom.clone())
        } else {
            self.files()
                .get(idx)
                .and_then(|f| f.get("name").and_then(|n| n.as_str()))
                .map(|s| s.to_string())
        }
    }

    pub fn set_file_title(&mut self, idx: usize, title: String) {
        self.custom_file_titles.insert(idx, title);
    }
}

// Editable channel type
#[derive(Debug, Clone)]
pub struct EditableChannel {
    pub id: String,
    pub name: String,
    pub messages: Vec<EditableMessage>,
    pub list_state: ListState,
}

impl EditableChannel {
    pub fn new(id: String, name: String, messages: Vec<serde_json::Value>) -> Self {
        let messages: Vec<EditableMessage> =
            messages.into_iter().map(EditableMessage::new).collect();
        let mut list_state = ListState::default();
        if !messages.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            id,
            name,
            messages,
            list_state,
        }
    }

    pub fn enabled_count(&self) -> usize {
        self.messages.iter().filter(|m| m.enabled).count()
    }

    pub fn next(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.messages.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn toggle_current(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(msg) = self.messages.get_mut(idx)
        {
            msg.enabled = !msg.enabled;
        }
    }

    pub fn toggle_collapse_current(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(msg) = self.messages.get_mut(idx)
        {
            msg.collapsed = !msg.collapsed;
        }
    }

    pub fn move_current_up(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && idx > 0
        {
            self.messages.swap(idx, idx - 1);
            self.list_state.select(Some(idx - 1));
        }
    }

    pub fn move_current_down(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && idx < self.messages.len() - 1
        {
            self.messages.swap(idx, idx + 1);
            self.list_state.select(Some(idx + 1));
        }
    }
}

// Editable channel list type
#[derive(Debug, Clone)]
pub struct EditableChannelList {
    pub channels: Vec<EditableChannel>,
    pub list_state: ListState,
    pub export_path: String,
}

impl EditableChannelList {
    pub fn new(raw_channels: Vec<(String, String, Vec<serde_json::Value>)>) -> Self {
        let mut channels: Vec<EditableChannel> = raw_channels
            .into_iter()
            .map(|(id, name, messages)| EditableChannel::new(id, name, messages))
            .collect();
        channels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let mut list_state = ListState::default();
        if !channels.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            channels,
            list_state,
            export_path: "./selected-conversations.json".to_string(),
        }
    }

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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

    pub fn move_current_up(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && idx > 0
        {
            self.channels.swap(idx, idx - 1);
            self.list_state.select(Some(idx - 1));
        }
    }

    pub fn move_current_down(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && idx < self.channels.len() - 1
        {
            self.channels.swap(idx, idx + 1);
            self.list_state.select(Some(idx + 1));
        }
    }

    pub fn to_export_data(&self) -> Vec<(String, String, Vec<serde_json::Value>)> {
        self.channels
            .iter()
            .map(|ch| {
                let filtered_messages: Vec<serde_json::Value> = ch
                    .messages
                    .iter()
                    .filter(|msg| msg.enabled)
                    .map(|msg| {
                        let mut exported = msg.original.clone();

                        if let Some(files) = exported.get("files").and_then(|f| f.as_array()) {
                            let filtered_files: Vec<serde_json::Value> = files
                                .iter()
                                .enumerate()
                                .filter(|(idx, _)| msg.selected_files.contains(idx))
                                .map(|(idx, f)| {
                                    let mut file = f.clone();
                                    if let Some(custom_title) = msg.custom_file_titles.get(&idx) {
                                        file["custom_title"] =
                                            serde_json::Value::String(custom_title.clone());
                                    }
                                    file
                                })
                                .collect();
                            if filtered_files.is_empty() {
                                exported.as_object_mut().map(|o| o.remove("files"));
                            } else {
                                exported["files"] = serde_json::Value::Array(filtered_files);
                            }
                        }

                        let all_links = msg.links();
                        let selected_links: Vec<serde_json::Value> = all_links
                            .iter()
                            .enumerate()
                            .filter(|(idx, _)| msg.selected_link_previews.contains(idx))
                            .map(|(idx, link)| {
                                let title =
                                    msg.get_link_title(idx).unwrap_or_else(|| link.title.clone());
                                serde_json::json!({
                                    "url": link.url,
                                    "title": title
                                })
                            })
                            .collect();

                        if !selected_links.is_empty() {
                            exported["selected_links"] = serde_json::Value::Array(selected_links);
                        }

                        if let Some(main_idx) = msg.main_link
                            && let Some(url) = msg.get_link_url(main_idx)
                        {
                            let title = msg.get_link_title(main_idx).unwrap_or_default();
                            exported["main_link"] = serde_json::json!({
                                "url": url,
                                "title": title
                            });
                        }

                        exported
                    })
                    .collect();
                (ch.id.clone(), ch.name.clone(), filtered_messages)
            })
            .collect()
    }
}

// Channel selection type
#[derive(Debug, Clone)]
pub struct ChannelSelection {
    pub channels: Vec<ChannelInfo>,
    pub selected: HashSet<String>,
    pub list_state: ListState,
}

impl ChannelSelection {
    pub fn new(channels: Vec<ChannelInfo>, saved_selection: Option<HashSet<String>>) -> Self {
        let mut channels = channels;
        channels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let selected = match saved_selection {
            Some(saved) if !saved.is_empty() => {
                let channel_ids: HashSet<_> = channels.iter().map(|c| c.id.clone()).collect();
                saved
                    .into_iter()
                    .filter(|id| channel_ids.contains(id))
                    .collect()
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

    pub fn toggle_current(&mut self) {
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

    pub fn select_all(&mut self) {
        self.selected = self.channels.iter().map(|c| c.id.clone()).collect();
    }

    pub fn select_none(&mut self) {
        self.selected.clear();
    }

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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

    pub fn selected_ids(&self) -> Vec<String> {
        self.selected.iter().cloned().collect()
    }
}

// Export task enum
#[derive(Debug, Clone)]
pub enum ExportTask {
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
    DownloadAttachments {
        conversations_path: String,
        output_path: String,
    },
    MarkdownExport {
        conversations_path: String,
        users_path: String,
        channels_path: String,
        output_path: String,
    },
    ExportEmojis {
        output_path: String,
        emojis_folder: String,
    },
}

// Screen enum
#[derive(Debug, Clone)]
pub enum Screen {
    MainMenu,
    ExportConversations {
        from_date: TextInput,
        to_date: TextInput,
        output_path: TextInput,
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
    EditConversationsPathInput {
        conversations_path: String,
        users_path: String,
        channels_path: String,
        active_field: EditConvPathField,
    },
    EditConversationsChannelList {
        channels: EditableChannelList,
        users: serde_json::Value,
        channel_data: serde_json::Value,
        editing_export_path: bool,
    },
    EditConversationsMessageList {
        channel_idx: usize,
        channels: EditableChannelList,
        users: serde_json::Value,
        channel_data: serde_json::Value,
    },
    EditConversationsMessageDetail {
        channel_idx: usize,
        message_idx: usize,
        channels: EditableChannelList,
        users: serde_json::Value,
        channel_data: serde_json::Value,
        attachment_list_state: ListState,
        editing_title: Option<(usize, TextInput)>,
    },
    DownloadAttachments {
        conversations_path: String,
        output_path: String,
        active_field: DownloadAttachmentsField,
    },
    MarkdownExport {
        conversations_path: String,
        users_path: String,
        channels_path: String,
        output_path: String,
        active_field: MarkdownExportField,
    },
    ExportEmojis {
        output_path: String,
        emojis_folder: String,
        active_field: ExportEmojisField,
    },
    Loading {
        message: String,
        progress: Option<(usize, usize, String)>,
    },
    Success {
        message: String,
    },
    Error {
        message: String,
    },
}

// Async result enum
pub enum AsyncResult {
    ExportComplete(std::result::Result<String, String>),
    ChannelsLoaded(std::result::Result<Vec<ChannelInfo>, String>),
}
