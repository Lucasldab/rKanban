use std::{fs, path::Path};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Card {
    pub title: String,
    pub description: String,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
        }
    }
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

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum InputMode {
    Normal,
    AddingCard,
    EditingCard { col: usize, card: usize },
    ViewingCard { col: usize, card: usize },
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
    pub input_buffer: String,
    pub desc_buffer: String,
    pub editing_desc: bool,
    pub show_help: bool,
    pub status_message: Option<String>,
}

impl App {
    const SAVE_FILE: &'static str = "board.json";

    pub fn new() -> Self {
        Self {
            selected_column: 0,
            quit: false,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            desc_buffer: String::new(),
            editing_desc: false,
            show_help: false,
            status_message: None,
            columns: vec![
                Column {
                    name: "Todo".into(),
                    selected: 0,
                    cards: vec![
                        Card::new("Write TUI"),
                        Card::new("Drink coffee"),
                    ],
                },
                Column {
                    name: "Doing".into(),
                    selected: 0,
                    cards: vec![Card::new("Learning Ratatui")],
                },
                Column {
                    name: "Done".into(),
                    selected: 0,
                    cards: vec![Card::new("Create project")],
                },
            ],
        }
    }

    fn snapshot(&self) -> BoardSnapshot {
        BoardSnapshot {
            columns: self.columns.clone(),
            selected_column: self.selected_column,
        }
    }

    pub fn load() -> Self {
        if Path::new(Self::SAVE_FILE).exists() {
            let data = fs::read_to_string(Self::SAVE_FILE).unwrap_or_default();
            if let Ok(snap) = serde_json::from_str::<BoardSnapshot>(&data) {
                let mut app = Self::new();
                app.columns = snap.columns;
                app.selected_column = snap
                    .selected_column
                    .min(app.columns.len().saturating_sub(1));
                for col in &mut app.columns {
                    col.clamp_selected();
                }
                return app;
            }
        }
        Self::new()
    }

    pub fn save(&mut self) {
        match serde_json::to_string_pretty(&self.snapshot()) {
            Ok(json) => {
                if fs::write(Self::SAVE_FILE, json).is_err() {
                    self.status_message = Some("⚠ Could not save board.json".into());
                }
            }
            Err(_) => self.status_message = Some("⚠ Serialisation error".into()),
        }
    }

    pub fn add_card(&mut self, title: String, description: String) {
        let col = &mut self.columns[self.selected_column];
        col.cards.push(Card { title, description });
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

    pub fn current_col(&self) -> &Column {
        &self.columns[self.selected_column]
    }

    pub fn current_card(&self) -> Option<&Card> {
        let col = self.current_col();
        col.cards.get(col.selected)
    }
}

#[derive(Serialize, Deserialize)]
struct BoardSnapshot {
    columns: Vec<Column>,
    selected_column: usize,
}
