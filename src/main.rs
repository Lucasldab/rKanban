mod app;
mod ui;

use std::{env, io};
use crate::app::{App, InputMode, PopupField};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let path = env::args().nth(1).unwrap_or_else(|| App::DEFAULT_FILE.to_string());
    let mut app = App::load(&path);
    let res = run(&mut terminal, &mut app);

    // Always save and restore terminal, even on error
    app.save();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        if app.quit {
            break;
        }

        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Ignore key-release events (important on Windows)
            if key.kind == KeyEventKind::Release {
                continue;
            }

            // Clear any previous status message on the next keypress
            app.status_message = None;

            match &app.input_mode.clone() {
                InputMode::Normal => handle_normal(app, key.code),
                InputMode::AddingCard => handle_input(app, key.code),
                InputMode::EditingCard { .. } => handle_input(app, key.code),
                InputMode::AddingColumn => handle_column_input(app, key.code),
                InputMode::RenamingColumn { .. } => handle_column_input(app, key.code),
                InputMode::DeletingColumn { .. } => handle_delete_confirm(app, key.code),
                InputMode::ViewingCard { .. } => {
                    if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                        app.input_mode = InputMode::Normal;
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_normal(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Char('?') => app.show_help = !app.show_help,

        // Column navigation
        KeyCode::Left => {
            app.selected_column = app.selected_column.saturating_sub(1);
        }
        KeyCode::Right => {
            app.selected_column =
                (app.selected_column + 1).min(app.columns.len().saturating_sub(1));
        }

        // Card navigation within the active column
        KeyCode::Up => {
            let col = &mut app.columns[app.selected_column];
            col.selected = col.selected.saturating_sub(1);
        }
        KeyCode::Down => {
            let col = &mut app.columns[app.selected_column];
            if !col.cards.is_empty() && col.selected + 1 < col.cards.len() {
                col.selected += 1;
            }
        }

        // Add card
        KeyCode::Char('a') => {
            app.input_mode = InputMode::AddingCard;
            app.reset_popup();
        }

        // Edit card
        KeyCode::Char('e') => {
            let col_idx = app.selected_column;
            let col = &app.columns[col_idx];
            if !col.cards.is_empty() {
                let card_idx = col.selected;
                let card = &col.cards[card_idx];
                app.input_buffer = card.title.clone();
                app.desc_buffer = card.description.clone();
                app.tags_buffer = card.tags.join(", ");
                app.title_cursor = app.input_buffer.len();
                app.desc_cursor = app.desc_buffer.len();
                app.tags_cursor = app.tags_buffer.len();
                app.focused_field = PopupField::Title;
                app.editing_desc = false;
                app.input_mode = InputMode::EditingCard {
                    col: col_idx,
                    card: card_idx,
                };
            }
        }

        // View card detail
        KeyCode::Char('v') | KeyCode::Enter => {
            let col_idx = app.selected_column;
            let col = &app.columns[col_idx];
            if !col.cards.is_empty() {
                app.input_mode = InputMode::ViewingCard {
                    col: col_idx,
                    card: col.selected,
                };
            }
        }

        // Move card between columns
        KeyCode::Char('h') => app.move_card_left(),
        KeyCode::Char('l') => app.move_card_right(),

        // Reorder card within column
        KeyCode::Char('K') => app.move_card_up(),
        KeyCode::Char('J') => app.move_card_down(),

        // Reorder column
        KeyCode::Char('H') => app.move_column_left(),
        KeyCode::Char('L') => app.move_column_right(),

        // Delete card
        KeyCode::Char('d') => app.delete_card(),

        // Column management
        KeyCode::Char('n') => {
            app.input_mode = InputMode::AddingColumn;
            app.input_buffer.clear();
            app.title_cursor = 0;
        }
        KeyCode::Char('r') => {
            let col_idx = app.selected_column;
            app.input_buffer = app.columns[col_idx].name.clone();
            app.title_cursor = app.input_buffer.len();
            app.input_mode = InputMode::RenamingColumn { col: col_idx };
        }
        KeyCode::Char('x') => {
            if app.columns.len() > 1 {
                let col_idx = app.selected_column;
                app.input_buffer.clear();
                app.title_cursor = 0;
                app.input_mode = InputMode::DeletingColumn { col: col_idx };
            } else {
                app.status_message = Some("⚠ Cannot delete the last column".into());
            }
        }

        _ => {}
    }
}

fn handle_input(app: &mut App, key: KeyCode) {
    match key {
        // Tab cycles: Title → Description → Tags → Title
        KeyCode::Tab => {
            app.focused_field = match app.focused_field {
                PopupField::Title => {
                    app.editing_desc = true;
                    PopupField::Description
                }
                PopupField::Description => {
                    app.editing_desc = false;
                    PopupField::Tags
                }
                PopupField::Tags => {
                    app.editing_desc = false;
                    PopupField::Title
                }
            };
        }

        // Enter: newline in description, commit in title/tags
        KeyCode::Enter => {
            if app.focused_field == PopupField::Description {
                // Insert a newline at cursor position
                let cursor = app.desc_cursor;
                app.desc_buffer.insert(cursor, '\n');
                app.desc_cursor = cursor + 1;
                return;
            }

            // Commit the card
            if !app.input_buffer.trim().is_empty() {
                let title = app.input_buffer.trim().to_string();
                let desc = app.desc_buffer.trim_end().to_string();
                let tags = app.parse_tags();

                match app.input_mode.clone() {
                    InputMode::AddingCard => {
                        app.add_card(title, desc, tags);
                    }
                    InputMode::EditingCard { col, card } => {
                        app.columns[col].cards[card].title = title;
                        app.columns[col].cards[card].description = desc;
                        app.columns[col].cards[card].tags = tags;
                        app.save();
                    }
                    _ => {}
                }
            }
            app.input_mode = InputMode::Normal;
            app.reset_popup();
        }

        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.reset_popup();
        }

        // Arrow keys: move cursor left/right in the focused field
        KeyCode::Left => {
            match app.focused_field {
                PopupField::Title => {
                    let mut c = app.title_cursor;
                    App::cursor_left(&app.input_buffer.clone(), &mut c);
                    app.title_cursor = c;
                }
                PopupField::Description => {
                    let mut c = app.desc_cursor;
                    App::cursor_left(&app.desc_buffer.clone(), &mut c);
                    app.desc_cursor = c;
                }
                PopupField::Tags => {
                    let mut c = app.tags_cursor;
                    App::cursor_left(&app.tags_buffer.clone(), &mut c);
                    app.tags_cursor = c;
                }
            }
        }
        KeyCode::Right => {
            match app.focused_field {
                PopupField::Title => {
                    let mut c = app.title_cursor;
                    App::cursor_right(&app.input_buffer.clone(), &mut c);
                    app.title_cursor = c;
                }
                PopupField::Description => {
                    let mut c = app.desc_cursor;
                    App::cursor_right(&app.desc_buffer.clone(), &mut c);
                    app.desc_cursor = c;
                }
                PopupField::Tags => {
                    let mut c = app.tags_cursor;
                    App::cursor_right(&app.tags_buffer.clone(), &mut c);
                    app.tags_cursor = c;
                }
            }
        }

        KeyCode::Backspace => {
            match app.focused_field {
                PopupField::Title => {
                    let (mut buf, mut cur) = (app.input_buffer.clone(), app.title_cursor);
                    App::delete_char_before(&mut buf, &mut cur);
                    app.input_buffer = buf;
                    app.title_cursor = cur;
                }
                PopupField::Description => {
                    let (mut buf, mut cur) = (app.desc_buffer.clone(), app.desc_cursor);
                    App::delete_char_before(&mut buf, &mut cur);
                    app.desc_buffer = buf;
                    app.desc_cursor = cur;
                }
                PopupField::Tags => {
                    let (mut buf, mut cur) = (app.tags_buffer.clone(), app.tags_cursor);
                    App::delete_char_before(&mut buf, &mut cur);
                    app.tags_buffer = buf;
                    app.tags_cursor = cur;
                }
            }
        }

        KeyCode::Char(c) => {
            match app.focused_field {
                PopupField::Title => {
                    let (mut buf, mut cur) = (app.input_buffer.clone(), app.title_cursor);
                    App::insert_char(&mut buf, &mut cur, c);
                    app.input_buffer = buf;
                    app.title_cursor = cur;
                }
                PopupField::Description => {
                    let (mut buf, mut cur) = (app.desc_buffer.clone(), app.desc_cursor);
                    App::insert_char(&mut buf, &mut cur, c);
                    app.desc_buffer = buf;
                    app.desc_cursor = cur;
                }
                PopupField::Tags => {
                    let (mut buf, mut cur) = (app.tags_buffer.clone(), app.tags_cursor);
                    App::insert_char(&mut buf, &mut cur, c);
                    app.tags_buffer = buf;
                    app.tags_cursor = cur;
                }
            }
        }

        _ => {}
    }
}

fn handle_column_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
            app.title_cursor = 0;
        }
        KeyCode::Enter => {
            let name = app.input_buffer.trim().to_string();
            if !name.is_empty() {
                match app.input_mode.clone() {
                    InputMode::AddingColumn => app.add_column(name),
                    InputMode::RenamingColumn { col } => app.rename_column(col, name),
                    _ => {}
                }
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
            app.title_cursor = 0;
        }
        KeyCode::Left => {
            let mut c = app.title_cursor;
            App::cursor_left(&app.input_buffer.clone(), &mut c);
            app.title_cursor = c;
        }
        KeyCode::Right => {
            let mut c = app.title_cursor;
            App::cursor_right(&app.input_buffer.clone(), &mut c);
            app.title_cursor = c;
        }
        KeyCode::Backspace => {
            let (mut buf, mut cur) = (app.input_buffer.clone(), app.title_cursor);
            App::delete_char_before(&mut buf, &mut cur);
            app.input_buffer = buf;
            app.title_cursor = cur;
        }
        KeyCode::Char(c) => {
            let (mut buf, mut cur) = (app.input_buffer.clone(), app.title_cursor);
            App::insert_char(&mut buf, &mut cur, c);
            app.input_buffer = buf;
            app.title_cursor = cur;
        }
        _ => {}
    }
}

fn handle_delete_confirm(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            if app.input_buffer.trim().eq_ignore_ascii_case("yes") {
                if let InputMode::DeletingColumn { col } = app.input_mode.clone() {
                    app.delete_column(col);
                }
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}
