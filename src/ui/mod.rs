use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    let column_constraints = vec![Constraint::Percentage(100 / app.columns.len() as u16); app.columns.len()];

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(column_constraints)
        .split(size);

    for (i, column) in app.columns.iter().enumerate() {
        let items: Vec<ListItem> = column
            .cards
            .iter()
            .map(|c| ListItem::new(c.title.clone()))
            .collect();

        let border_style = if i == app.selected_column {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(column.name.clone())
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );

        f.render_widget(list, chunks[i]);
    }
}
