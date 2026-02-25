mod app;
mod ui;

use std::io;
use crate::app::{App, InputMode};

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

    let mut app = App::load();
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
                InputMode::AddingCard => handle_input(app, key.code, false),
                InputMode::EditingCard { col, card } => {
                    handle_input(app, key.code, true);
                    // The handler will have already committed / cancelled
                }
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
            app.input_buffer.clear();
            app.desc_buffer.clear();
            app.editing_desc = false;
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

        // Delete card
        KeyCode::Char('d') => app.delete_card(),

        _ => {}
    }
}

fn handle_input(app: &mut App, key: KeyCode, is_edit: bool) {
    match key {
        KeyCode::Tab => {
            // Switch focus between title and description fields
            app.editing_desc = !app.editing_desc;
        }
        KeyCode::Enter => {
            // Commit: only require a non-empty title
            if !app.input_buffer.trim().is_empty() {
                match app.input_mode.clone() {
                    InputMode::AddingCard => {
                        app.add_card(
                            app.input_buffer.trim().to_string(),
                            app.desc_buffer.trim().to_string(),
                        );
                    }
                    InputMode::EditingCard { col, card } => {
                        app.columns[col].cards[card].title =
                            app.input_buffer.trim().to_string();
                        app.columns[col].cards[card].description =
                            app.desc_buffer.trim().to_string();
                        app.save();
                    }
                    _ => {}
                }
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
            app.desc_buffer.clear();
            app.editing_desc = false;
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
            app.desc_buffer.clear();
            app.editing_desc = false;
        }
        KeyCode::Backspace => {
            if app.editing_desc {
                app.desc_buffer.pop();
            } else {
                app.input_buffer.pop();
            }
        }
        KeyCode::Char(c) => {
            if app.editing_desc {
                app.desc_buffer.push(c);
            } else {
                app.input_buffer.push(c);
            }
        }
        _ => {}
    }
}
