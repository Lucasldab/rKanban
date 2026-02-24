#[derive(Clone)]
pub struct Card {
    pub title: String,
}

#[derive(Clone)]
pub struct Column {
    pub name: String,
    pub cards: Vec<Card>,
    pub selected: usize,
}

pub struct App {
    pub columns: Vec<Column>,
    pub selected_column: usize,
    pub quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            selected_column: 0,
            quit: false,
            columns: vec![
                Column {
                    name: "Todo".into(),
                    selected: 0,
                    cards: vec![
                        Card { title: "Write TUI".into() },
                        Card { title: "Drink coffee".into() },
                    ],
                },
                Column {
                    name: "Doing".into(),
                    selected: 0,
                    cards: vec![
                        Card { title: "Learning Ratatui".into() },
                    ],
                },
                Column {
                    name: "Done".into(),
                    selected: 0,
                    cards: vec![
                        Card { title: "Create project".into() },
                    ],
                },
            ],
        }
    }
}
