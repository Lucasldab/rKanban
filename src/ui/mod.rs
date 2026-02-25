use crate::app::{App, InputMode, PopupField};
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
const TAG_FG: Color = Color::Magenta;

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
        InputMode::AddingColumn => {
            draw_column_name_popup(f, app, area, false);
        }
        InputMode::RenamingColumn { .. } => {
            draw_column_name_popup(f, app, area, true);
        }
        InputMode::DeletingColumn { col } => {
            draw_column_delete_popup(f, app, area, *col);
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
                let base_style = if selected {
                    Style::default()
                        .fg(SELECTED_FG)
                        .bg(SELECTED_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                // Show a bullet and optionally a description hint
                let desc_hint = if !c.description.is_empty() { " ✎" } else { "" };
                let title_line = Line::from(Span::styled(
                    format!(" {}{}", c.title, desc_hint),
                    base_style,
                ));

                // Tags inline to the right of the title
                if c.tags.is_empty() {
                    ListItem::new(title_line)
                } else {
                    let tag_style = if selected {
                        Style::default().fg(SELECTED_FG).bg(SELECTED_BG)
                    } else {
                        Style::default().fg(TAG_FG)
                    };
                    let tag_span = Span::styled(
                        format!(" [{}]", c.tags.join(", ")),
                        tag_style,
                    );
                    let mut spans = title_line.spans;
                    spans.push(tag_span);
                    ListItem::new(Line::from(spans))
                }
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
                "  ←/→ col  ↑/↓ card  a add  e edit  v view  h/l move col  H/L reorder col  J/K reorder card  d del  n/r/x col  ? help  q quit"
            }
            InputMode::AddingCard | InputMode::EditingCard { .. } => {
                "  Tab: next field  ←/→: cursor  Enter: newline(desc)/confirm  Esc: cancel"
            }
            InputMode::AddingColumn | InputMode::RenamingColumn { .. } => {
                "  ←/→: cursor  Enter: confirm  Esc: cancel"
            }
            InputMode::DeletingColumn { .. } => {
                "  Type yes and press Enter to confirm deletion  Esc: cancel"
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
    let popup_area = centered_rect(60, 50, area);
    let title = match &app.input_mode {
        InputMode::AddingCard => " New Card ",
        InputMode::EditingCard { .. } => " Edit Card ",
        _ => "",
    };
    let outer_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black))
        .border_style(Style::default().fg(ACCENT));
    let inner_area = outer_block.inner(popup_area);
    f.render_widget(Clear, popup_area);
    f.render_widget(outer_block, popup_area);

    // Layout: title(3) + description(flexible, min 3) + tags(3)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title field
            Constraint::Min(3),    // description field
            Constraint::Length(3), // tags field
        ])
        .split(inner_area);

    // ── Title field ───────────────────────────────────────────────────────────
    let title_focused = app.focused_field == PopupField::Title;
    let title_style = if title_focused {
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

    // ── Description field (multiline) ─────────────────────────────────────────
    let desc_focused = app.focused_field == PopupField::Description;
    let desc_style = if desc_focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(SUBTLE)
    };
    let desc_field = Paragraph::new(app.desc_buffer.as_str())
        .block(
            Block::default()
                .title(" Description (Enter for newline) ")
                .borders(Borders::ALL)
                .border_style(desc_style),
        )
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false });
    f.render_widget(desc_field, rows[1]);

    // ── Tags field ────────────────────────────────────────────────────────────
    let tags_focused = app.focused_field == PopupField::Tags;
    let tags_style = if tags_focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(SUBTLE)
    };
    let tags_field = Paragraph::new(app.tags_buffer.as_str())
        .block(
            Block::default()
                .title(" Tags (comma-separated) ")
                .borders(Borders::ALL)
                .border_style(tags_style),
        )
        .style(Style::default().fg(TAG_FG));
    f.render_widget(tags_field, rows[2]);

    // ── Cursor placement ──────────────────────────────────────────────────────
    match app.focused_field {
        PopupField::Title => {
            let x = rows[0].x + 1 + cursor_display_col(&app.input_buffer, app.title_cursor) as u16;
            let y = rows[0].y + 1;
            f.set_cursor_position((x.min(rows[0].right() - 2), y));
        }
        PopupField::Description => {
            let inner_w = rows[1].width.saturating_sub(2) as usize;
            let (vis_col, vis_row) = cursor_visual_pos(&app.desc_buffer, app.desc_cursor, inner_w);
            let x = rows[1].x + 1 + vis_col as u16;
            let y = rows[1].y + 1 + vis_row as u16;
            f.set_cursor_position((x.min(rows[1].right() - 2), y.min(rows[1].bottom() - 2)));
        }
        PopupField::Tags => {
            let x = rows[2].x + 1 + cursor_display_col(&app.tags_buffer, app.tags_cursor) as u16;
            let y = rows[2].y + 1;
            f.set_cursor_position((x.min(rows[2].right() - 2), y));
        }
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
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        // Split: description area + optional tags line at bottom
        let has_tags = !c.tags.is_empty();
        let constraints = if has_tags {
            vec![Constraint::Min(1), Constraint::Length(1)]
        } else {
            vec![Constraint::Min(1)]
        };
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let desc = if c.description.is_empty() {
            "(no description)".to_string()
        } else {
            c.description.clone()
        };
        let para = Paragraph::new(desc)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));
        f.render_widget(para, sections[0]);

        if has_tags {
            let tag_line = Paragraph::new(Line::from(vec![
                Span::styled("Tags: ", Style::default().fg(SUBTLE)),
                Span::styled(c.tags.join(", "), Style::default().fg(TAG_FG)),
            ]));
            f.render_widget(tag_line, sections[1]);
        }
    }
}

// ── Help overlay ──────────────────────────────────────────────────────────────
fn draw_help(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(50, 70, area);
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
        Line::from("  J / K   Reorder card down / up"),
        Line::from(""),
        Line::from(Span::styled(" Popup Editing", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from("  Tab     Cycle fields (Title → Desc → Tags)"),
        Line::from("  ←/→     Move cursor within field"),
        Line::from("  Enter   Newline in desc / confirm card"),
        Line::from("  Esc     Cancel"),
        Line::from(""),
        Line::from(Span::styled(" Column Management", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from("  n       Add new column"),
        Line::from("  r       Rename selected column"),
        Line::from("  x       Delete selected column"),
        Line::from("  H / L   Reorder column left / right"),
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

// ── Column name popup ────────────────────────────────────────────────────────
fn draw_column_name_popup(f: &mut Frame, app: &App, area: Rect, is_rename: bool) {
    let popup_area = centered_rect(50, 20, area);
    let title = if is_rename { " Rename Column " } else { " New Column " };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);

    let field = Paragraph::new(app.input_buffer.as_str())
        .block(
            Block::default()
                .title(" Column Name ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT)),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(field, inner);

    let x = inner.x + 1 + cursor_display_col(&app.input_buffer, app.title_cursor) as u16;
    let y = inner.y + 1;
    f.set_cursor_position((x.min(inner.right() - 2), y));
}

// ── Column delete confirm popup ───────────────────────────────────────────────
fn draw_column_delete_popup(f: &mut Frame, app: &App, area: Rect, col: usize) {
    let popup_area = centered_rect(50, 25, area);
    let col_name = app.columns.get(col).map(|c| c.name.as_str()).unwrap_or("?");
    let block = Block::default()
        .title(" Delete Column ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(WARNING))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(3)])
        .split(inner);

    let warning = Paragraph::new(Line::from(vec![
        Span::styled("Delete ", Style::default().fg(Color::White)),
        Span::styled(format!("'{}'" , col_name), Style::default().fg(WARNING).add_modifier(Modifier::BOLD)),
        Span::styled(" and all its cards?", Style::default().fg(Color::White)),
    ]));
    f.render_widget(warning, rows[0]);

    let confirm_field = Paragraph::new(app.input_buffer.as_str())
        .block(
            Block::default()
                .title(" Type yes to confirm ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(WARNING)),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(confirm_field, rows[1]);

    let x = rows[1].x + 1 + app.input_buffer.len() as u16;
    let y = rows[1].y + 1;
    f.set_cursor_position((x.min(rows[1].right() - 2), y));
}

// ── Cursor helpers ────────────────────────────────────────────────────────────

/// Display column (char count) for a byte offset into a single-line buffer
fn cursor_display_col(buf: &str, cursor: usize) -> usize {
    buf[..cursor.min(buf.len())].chars().count()
}

/// Given a byte offset into a multiline (possibly wrapped) buffer,
/// return the (visual_col, visual_row) to place the terminal cursor.
fn cursor_visual_pos(buf: &str, cursor: usize, inner_w: usize) -> (usize, usize) {
    let w = inner_w.max(1);
    let safe = cursor.min(buf.len());
    let before = &buf[..safe];
    let mut vis_row = 0usize;
    let mut vis_col = 0usize;

    for (li, logical_line) in before.split('\n').enumerate() {
        let char_count = logical_line.chars().count();
        if li > 0 {
            // previous logical line added its wrap rows already; now add the newline row
            vis_row += 1;
        }
        // extra visual rows from wrapping within this logical line
        vis_row += char_count / w;
        vis_col = char_count % w;
    }

    (vis_col, vis_row)
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
