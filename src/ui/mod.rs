use crate::app::{App, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

// ── Colour palette ────────────────────────────────────────────────────────────
const ACCENT: Color = Color::Cyan;
const SELECTED_BG: Color = Color::Cyan;
const SELECTED_FG: Color = Color::Black;
const SUBTLE: Color = Color::DarkGray;
const WARNING: Color = Color::Yellow;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Reserve 1 row at the bottom for the status/hint bar
    let main_and_bar = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    draw_columns(f, app, main_and_bar[0]);
    draw_statusbar(f, app, main_and_bar[1]);

    // Overlays (rendered last so they appear on top)
    match &app.input_mode {
        InputMode::AddingCard | InputMode::EditingCard { .. } => {
            draw_input_popup(f, app, area);
        }
        InputMode::ViewingCard { col, card } => {
            draw_card_detail(f, app, area, *col, *card);
        }
        InputMode::Normal => {}
    }

    if app.show_help {
        draw_help(f, area);
    }
}

// ── Board columns ─────────────────────────────────────────────────────────────

fn draw_columns(f: &mut Frame, app: &App, area: Rect) {
    let n = app.columns.len() as u16;
    let constraints: Vec<Constraint> = (0..n)
        .map(|_| Constraint::Ratio(1, n as u32))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, col) in app.columns.iter().enumerate() {
        let is_active = i == app.selected_column;

        let border_style = if is_active {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SUBTLE)
        };

        let title_str = format!(" {} ({}) ", col.name, col.cards.len());

        let items: Vec<ListItem> = col
            .cards
            .iter()
            .enumerate()
            .map(|(j, c)| {
                let selected = is_active && j == col.selected;
                let style = if selected {
                    Style::default()
                        .fg(SELECTED_FG)
                        .bg(SELECTED_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                // Show a bullet and optionally a description hint
                let label = if c.description.is_empty() {
                    format!(" • {}", c.title)
                } else {
                    format!(" • {} ✎", c.title)
                };
                ListItem::new(label).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(title_str.as_str())
                .borders(Borders::ALL)
                .border_style(border_style),
        );

        f.render_widget(list, chunks[i]);
    }
}

// ── Status / hint bar ─────────────────────────────────────────────────────────

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(msg) = &app.status_message {
        Span::styled(msg.clone(), Style::default().fg(WARNING))
    } else {
        let hint = match &app.input_mode {
            InputMode::Normal => {
                "  ←/→ col  ↑/↓ card  a add  e edit  v view  h/l move  d delete  ? help  q quit"
            }
            InputMode::AddingCard | InputMode::EditingCard { .. } => {
                "  Tab: switch field  Enter: confirm  Esc: cancel"
            }
            InputMode::ViewingCard { .. } => "  Esc: back",
        };
        Span::styled(hint, Style::default().fg(SUBTLE))
    };

    let para = Paragraph::new(Line::from(text));
    f.render_widget(para, area);
}

// ── Add / Edit popup ──────────────────────────────────────────────────────────

fn draw_input_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 40, area);

    let title = match &app.input_mode {
        InputMode::AddingCard => " New Card ",
        InputMode::EditingCard { .. } => " Edit Card ",
        _ => "",
    };

    // Split popup into title input, description input, and a hint row
    let inner = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black))
        .border_style(Style::default().fg(ACCENT));

    let inner_area = inner.inner(popup_area);
    f.render_widget(Clear, popup_area);
    f.render_widget(inner, popup_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title field
            Constraint::Min(3),    // description field
            Constraint::Length(1), // spacer / hint
        ])
        .split(inner_area);

    // Title field
    let title_style = if !app.editing_desc {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(SUBTLE)
    };
    let title_field = Paragraph::new(app.input_buffer.as_str())
        .block(
            Block::default()
                .title(" Title ")
                .borders(Borders::ALL)
                .border_style(title_style),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(title_field, rows[0]);

    // Description field
    let desc_style = if app.editing_desc {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(SUBTLE)
    };
    let desc_field = Paragraph::new(app.desc_buffer.as_str())
        .block(
            Block::default()
                .title(" Description (optional) ")
                .borders(Borders::ALL)
                .border_style(desc_style),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(desc_field, rows[1]);

    // Place the cursor
    if !app.editing_desc {
        let x = rows[0].x + 1 + app.input_buffer.len() as u16;
        let y = rows[0].y + 1;
        f.set_cursor_position((x.min(rows[0].right() - 2), y));
    } else {
        let x = rows[1].x + 1 + app.desc_buffer.len() as u16;
        let y = rows[1].y + 1;
        f.set_cursor_position((x.min(rows[1].right() - 2), y));
    }
}

// ── Card detail view ──────────────────────────────────────────────────────────

fn draw_card_detail(f: &mut Frame, app: &App, area: Rect, col: usize, card: usize) {
    if let Some(c) = app.columns.get(col).and_then(|c| c.cards.get(card)) {
        let popup_area = centered_rect(70, 50, area);
        f.render_widget(Clear, popup_area);

        let col_name = &app.columns[col].name;
        let block = Block::default()
            .title(format!(" {} → {} ", col_name, c.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(Color::Black));

        let desc = if c.description.is_empty() {
            "(no description)".to_string()
        } else {
            c.description.clone()
        };

        let para = Paragraph::new(desc)
            .block(block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));

        f.render_widget(para, popup_area);
    }
}

// ── Help overlay ──────────────────────────────────────────────────────────────

fn draw_help(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(50, 60, area);
    f.render_widget(Clear, popup_area);

    let help_text = Text::from(vec![
        Line::from(Span::styled(" Navigation", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from("  ←/→     Select column"),
        Line::from("  ↑/↓     Select card"),
        Line::from(""),
        Line::from(Span::styled(" Card Actions", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from("  a       Add new card"),
        Line::from("  e       Edit selected card"),
        Line::from("  v       View card details"),
        Line::from("  d       Delete selected card"),
        Line::from("  h / l   Move card left / right"),
        Line::from(""),
        Line::from(Span::styled(" General", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from("  ?       Toggle this help"),
        Line::from("  q       Quit (auto-saves)"),
    ]);

    let para = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(para, popup_area);
}

// ── Layout helpers ────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
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
        .split(vert[1])[1]
}
