use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::Paragraph,
    Frame,
};

/// A reusable text input widget with cursor support
#[derive(Debug, Clone)]
pub struct TextInput {
    text: String,
    cursor: usize, // Cursor position in characters (not bytes)
}

impl TextInput {
    pub fn new(text: String) -> Self {
        let cursor = text.chars().count();
        Self { text, cursor }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    #[allow(dead_code)]
    pub fn into_text(self) -> String {
        self.text
    }

    #[allow(dead_code)]
    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    /// Handle a key event, returns true if the event was handled
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            // Cursor movement
            (KeyCode::Left, KeyModifiers::NONE) => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                true
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                if self.cursor < self.text.chars().count() {
                    self.cursor += 1;
                }
                true
            }
            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.cursor = 0;
                true
            }
            (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.cursor = self.text.chars().count();
                true
            }
            // Word movement (Ctrl+Left/Right)
            (KeyCode::Left, KeyModifiers::CONTROL) => {
                self.move_word_left();
                true
            }
            (KeyCode::Right, KeyModifiers::CONTROL) => {
                self.move_word_right();
                true
            }
            // Deletion
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                if self.cursor > 0 {
                    let byte_pos = self.char_to_byte_pos(self.cursor - 1);
                    let next_byte_pos = self.char_to_byte_pos(self.cursor);
                    self.text.replace_range(byte_pos..next_byte_pos, "");
                    self.cursor -= 1;
                }
                true
            }
            (KeyCode::Delete, KeyModifiers::NONE) => {
                let len = self.text.chars().count();
                if self.cursor < len {
                    let byte_pos = self.char_to_byte_pos(self.cursor);
                    let next_byte_pos = self.char_to_byte_pos(self.cursor + 1);
                    self.text.replace_range(byte_pos..next_byte_pos, "");
                }
                true
            }
            // Delete word backward (Ctrl+Backspace or Ctrl+W)
            (KeyCode::Backspace, KeyModifiers::CONTROL)
            | (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                let old_cursor = self.cursor;
                self.move_word_left();
                if self.cursor < old_cursor {
                    let start_byte = self.char_to_byte_pos(self.cursor);
                    let end_byte = self.char_to_byte_pos(old_cursor);
                    self.text.replace_range(start_byte..end_byte, "");
                }
                true
            }
            // Clear line (Ctrl+U)
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.text.clear();
                self.cursor = 0;
                true
            }
            // Kill to end of line (Ctrl+K)
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                let byte_pos = self.char_to_byte_pos(self.cursor);
                self.text.truncate(byte_pos);
                true
            }
            // Character input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                let byte_pos = self.char_to_byte_pos(self.cursor);
                self.text.insert(byte_pos, c);
                self.cursor += 1;
                true
            }
            _ => false,
        }
    }

    fn char_to_byte_pos(&self, char_pos: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    fn move_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.text.chars().collect();
        let mut pos = self.cursor - 1;
        // Skip whitespace
        while pos > 0 && chars[pos].is_whitespace() {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        self.cursor = pos;
    }

    fn move_word_right(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        let len = chars.len();
        if self.cursor >= len {
            return;
        }
        let mut pos = self.cursor;
        // Skip current word
        while pos < len && !chars[pos].is_whitespace() {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }
        self.cursor = pos;
    }

    /// Render the input and return cursor screen position
    pub fn render(&self, f: &mut Frame, area: Rect, style: Style) {
        let paragraph = Paragraph::new(self.text.as_str()).style(style);
        f.render_widget(paragraph, area);

        // Set cursor position
        let cursor_x = area.x + self.cursor as u16;
        let cursor_y = area.y;
        if cursor_x < area.x + area.width {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new(String::new())
    }
}
