use std::{fs, path::Path};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Card {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}


#[derive(Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub cards: Vec<Card>,
    pub selected: usize,
}

impl Column {
    pub fn clamp_selected(&mut self) {
        if self.cards.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.cards.len() {
            self.selected = self.cards.len() - 1;
        }
    }
}

/// Which field is focused in the add/edit popup
#[derive(Clone, PartialEq)]
pub enum PopupField {
    Title,
    Description,
    Tags,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum InputMode {
    Normal,
    AddingCard,
    EditingCard { col: usize, card: usize },
    ViewingCard { col: usize, card: usize },
    AddingColumn,
    RenamingColumn { col: usize },
    DeletingColumn { col: usize },
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::Normal
    }
}

pub struct App {
    pub columns: Vec<Column>,
    pub selected_column: usize,
    pub quit: bool,
    pub input_mode: InputMode,

    // Buffers for the add/edit popup
    pub input_buffer: String,   // title
    pub desc_buffer: String,    // description (may contain \n)
    pub tags_buffer: String,    // comma-separated tags

    // Which field is focused
    pub focused_field: PopupField,

    // Cursor positions (byte offsets into each buffer)
    pub title_cursor: usize,
    pub desc_cursor: usize,
    pub tags_cursor: usize,

    /// Legacy flag kept so ui.rs can check focus without matching PopupField
    pub editing_desc: bool,

    pub show_help: bool,
    pub save_path: String,
    pub status_message: Option<String>,
}

impl App {
    pub const DEFAULT_FILE: &'static str = "board.json";

    pub fn new() -> Self {
        Self {
            selected_column: 0,
            quit: false,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            desc_buffer: String::new(),
            tags_buffer: String::new(),
            focused_field: PopupField::Title,
            title_cursor: 0,
            desc_cursor: 0,
            tags_cursor: 0,
            editing_desc: false,
            show_help: false,
            save_path: Self::DEFAULT_FILE.to_string(),
            status_message: None,
            columns: vec![
                Column { name: "Todo".into(),  selected: 0, cards: vec![] },
                Column { name: "Doing".into(), selected: 0, cards: vec![] },
                Column { name: "Done".into(),  selected: 0, cards: vec![] },
            ],
        }
    }

    /// Parse tags_buffer into a Vec<String>, trimming whitespace and ignoring empty entries
    pub fn parse_tags(&self) -> Vec<String> {
        self.tags_buffer
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect()
    }

    fn snapshot(&self) -> BoardSnapshot {
        BoardSnapshot {
            columns: self.columns.clone(),
            selected_column: self.selected_column,
        }
    }

    pub fn load(path: &str) -> Self {
        if Path::new(path).exists() {
            let data = fs::read_to_string(path).unwrap_or_default();
            if let Ok(snap) = serde_json::from_str::<BoardSnapshot>(&data) {
                let mut app = Self::new();
                app.columns = snap.columns;
                app.selected_column = snap
                    .selected_column
                    .min(app.columns.len().saturating_sub(1));
                for col in &mut app.columns {
                    col.clamp_selected();
                }
                app.save_path = path.to_string();
                return app;
            }
        }
        let mut app = Self::new();
        app.save_path = path.to_string();
        app
    }

    pub fn save(&mut self) {
        match serde_json::to_string_pretty(&self.snapshot()) {
            Ok(json) => {
                if fs::write(&self.save_path, json).is_err() {
                    self.status_message = Some(format!("⚠ Could not save {}", self.save_path));
                }
            }
            Err(_) => self.status_message = Some("⚠ Serialisation error".into()),
        }
    }

    pub fn add_card(&mut self, title: String, description: String, tags: Vec<String>) {
        let col = &mut self.columns[self.selected_column];
        col.cards.push(Card { title, description, tags });
        col.selected = col.cards.len() - 1;
        self.save();
    }

    pub fn delete_card(&mut self) {
        let col = &mut self.columns[self.selected_column];
        if !col.cards.is_empty() {
            col.cards.remove(col.selected);
            col.clamp_selected();
            self.save();
        }
    }

    pub fn move_card_up(&mut self) {
        let col = &mut self.columns[self.selected_column];
        if col.selected == 0 || col.cards.len() < 2 { return; }
        col.cards.swap(col.selected, col.selected - 1);
        col.selected -= 1;
        self.save();
    }

    pub fn move_card_down(&mut self) {
        let col = &mut self.columns[self.selected_column];
        if col.cards.len() < 2 || col.selected + 1 >= col.cards.len() { return; }
        col.cards.swap(col.selected, col.selected + 1);
        col.selected += 1;
        self.save();
    }

    pub fn move_column_left(&mut self) {
        let i = self.selected_column;
        if i == 0 { return; }
        self.columns.swap(i, i - 1);
        self.selected_column -= 1;
        self.save();
    }

    pub fn move_column_right(&mut self) {
        let i = self.selected_column;
        if i + 1 >= self.columns.len() { return; }
        self.columns.swap(i, i + 1);
        self.selected_column += 1;
        self.save();
    }

    pub fn move_card_left(&mut self) {
        if self.selected_column == 0 { return; }
        self.move_card_to(self.selected_column - 1);
    }

    pub fn move_card_right(&mut self) {
        if self.selected_column + 1 >= self.columns.len() { return; }
        self.move_card_to(self.selected_column + 1);
    }

    fn move_card_to(&mut self, dst: usize) {
        let src = self.selected_column;
        if self.columns[src].cards.is_empty() { return; }
        let idx = self.columns[src].selected;
        let card = self.columns[src].cards.remove(idx);
        self.columns[src].clamp_selected();
        self.columns[dst].cards.push(card);
        self.columns[dst].selected = self.columns[dst].cards.len() - 1;
        self.selected_column = dst;
        self.save();
    }

    pub fn add_column(&mut self, name: String) {
        self.columns.push(Column {
            name,
            cards: Vec::new(),
            selected: 0,
        });
        self.selected_column = self.columns.len() - 1;
        self.save();
    }

    pub fn rename_column(&mut self, col: usize, name: String) {
        if let Some(c) = self.columns.get_mut(col) {
            c.name = name;
            self.save();
        }
    }

    pub fn delete_column(&mut self, col: usize) {
        if self.columns.len() <= 1 { return; } // always keep at least one
        self.columns.remove(col);
        self.selected_column = col.saturating_sub(1).min(self.columns.len() - 1);
        self.save();
    }

    /// Clear all popup buffers and reset focus to Title
    pub fn reset_popup(&mut self) {
        self.input_buffer.clear();
        self.desc_buffer.clear();
        self.tags_buffer.clear();
        self.title_cursor = 0;
        self.desc_cursor = 0;
        self.tags_cursor = 0;
        self.focused_field = PopupField::Title;
        self.editing_desc = false;
    }

    // ── Cursor helpers ──────────────────────────────────────────────────────

    /// Move cursor left by one char in `buf`, updating `cursor` in place
    pub fn cursor_left(buf: &str, cursor: &mut usize) {
        if *cursor == 0 { return; }
        // Step back one UTF-8 char
        *cursor -= 1;
        while *cursor > 0 && !buf.is_char_boundary(*cursor) {
            *cursor -= 1;
        }
    }

    /// Move cursor right by one char in `buf`, updating `cursor` in place
    pub fn cursor_right(buf: &str, cursor: &mut usize) {
        if *cursor >= buf.len() { return; }
        *cursor += 1;
        while *cursor < buf.len() && !buf.is_char_boundary(*cursor) {
            *cursor += 1;
        }
    }

    /// Insert a char at `cursor` position in `buf`
    pub fn insert_char(buf: &mut String, cursor: &mut usize, ch: char) {
        buf.insert(*cursor, ch);
        *cursor += ch.len_utf8();
    }

    /// Delete char before `cursor` (backspace)
    pub fn delete_char_before(buf: &mut String, cursor: &mut usize) {
        if *cursor == 0 { return; }
        let mut start = *cursor - 1;
        while start > 0 && !buf.is_char_boundary(start) {
            start -= 1;
        }
        buf.drain(start..*cursor);
        *cursor = start;
    }
}

#[derive(Serialize, Deserialize)]
struct BoardSnapshot {
    columns: Vec<Column>,
    selected_column: usize,
}
