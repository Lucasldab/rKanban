use ratatui::{
    layout::Alignment,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, _app: &App) {
    let area = f.area();

    let block = Block::default()
        .title("rkanban")
        .borders(Borders::ALL);

    let text = Paragraph::new("Kanban TUI – press q to quit")
        .alignment(Alignment::Center)
        .block(block);

    f.render_widget(text, area);
}
