mod app;
mod config;
mod ui;

use std::{env, io};
use crate::app::{App, InputMode, PopupField};
use crate::config::{Config, Key};

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

    let cfg = config::load();
    let path = env::args().nth(1).unwrap_or_else(|| App::DEFAULT_FILE.to_string());
    let mut app = App::load(&path);
    let res = run(&mut terminal, &mut app, &cfg);

    app.save();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    cfg: &Config,
) -> io::Result<()> {
    loop {
        if app.quit { break; }
        terminal.draw(|f| ui::draw(f, app, cfg))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release { continue; }
            app.status_message = None;

            match &app.input_mode.clone() {
                InputMode::Normal                => handle_normal(app, key.code, cfg),
                InputMode::AddingCard            => handle_input(app, key.code, cfg),
                InputMode::EditingCard { .. }    => handle_input(app, key.code, cfg),
                InputMode::AddingColumn          => handle_column_input(app, key.code, cfg),
                InputMode::RenamingColumn { .. } => handle_column_input(app, key.code, cfg),
                InputMode::DeletingColumn { .. } => handle_delete_confirm(app, key.code, cfg),
                InputMode::ViewingCard { .. }    => {
                    if cfg.keys.popup_cancel.matches(&key.code)
                        || Key::Char('q').matches(&key.code)
                    {
                        app.input_mode = InputMode::Normal;
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_normal(app: &mut App, key: KeyCode, cfg: &Config) {
    let k = &cfg.keys;
    if k.quit.matches(&key) {
        app.quit = true;
    } else if k.help.matches(&key) {
        app.show_help = !app.show_help;
    } else if k.nav_left.matches(&key) {
        app.selected_column = app.selected_column.saturating_sub(1);
    } else if k.nav_right.matches(&key) {
        app.selected_column = (app.selected_column + 1).min(app.columns.len().saturating_sub(1));
    } else if k.nav_up.matches(&key) {
        let col = &mut app.columns[app.selected_column];
        col.selected = col.selected.saturating_sub(1);
    } else if k.nav_down.matches(&key) {
        let col = &mut app.columns[app.selected_column];
        if !col.cards.is_empty() && col.selected + 1 < col.cards.len() {
            col.selected += 1;
        }
    } else if k.add_card.matches(&key) {
        app.input_mode = InputMode::AddingCard;
        app.reset_popup();
    } else if k.edit_card.matches(&key) {
        let col_idx = app.selected_column;
        let col = &app.columns[col_idx];
        if !col.cards.is_empty() {
            let card_idx = col.selected;
            let card = &col.cards[card_idx];
            app.input_buffer = card.title.clone();
            app.desc_buffer  = card.description.clone();
            app.tags_buffer  = card.tags.join(", ");
            app.title_cursor = app.input_buffer.len();
            app.desc_cursor  = app.desc_buffer.len();
            app.tags_cursor  = app.tags_buffer.len();
            app.focused_field = PopupField::Title;
            app.editing_desc  = false;
            app.input_mode = InputMode::EditingCard { col: col_idx, card: card_idx };
        }
    } else if k.view_card.matches(&key) || k.popup_confirm.matches(&key) {
        let col_idx = app.selected_column;
        let col = &app.columns[col_idx];
        if !col.cards.is_empty() {
            app.input_mode = InputMode::ViewingCard { col: col_idx, card: col.selected };
        }
    } else if k.delete_card.matches(&key) {
        app.delete_card();
    } else if k.move_card_left.matches(&key) {
        app.move_card_left();
    } else if k.move_card_right.matches(&key) {
        app.move_card_right();
    } else if k.reorder_up.matches(&key) {
        app.move_card_up();
    } else if k.reorder_down.matches(&key) {
        app.move_card_down();
    } else if k.add_column.matches(&key) {
        app.input_mode = InputMode::AddingColumn;
        app.input_buffer.clear();
        app.title_cursor = 0;
    } else if k.rename_column.matches(&key) {
        let col_idx = app.selected_column;
        app.input_buffer = app.columns[col_idx].name.clone();
        app.title_cursor = app.input_buffer.len();
        app.input_mode = InputMode::RenamingColumn { col: col_idx };
    } else if k.delete_column.matches(&key) {
        if app.columns.len() > 1 {
            let col_idx = app.selected_column;
            app.input_buffer.clear();
            app.title_cursor = 0;
            app.input_mode = InputMode::DeletingColumn { col: col_idx };
        } else {
            app.status_message = Some("⚠ Cannot delete the last column".into());
        }
    } else if k.reorder_col_left.matches(&key) {
        app.move_column_left();
    } else if k.reorder_col_right.matches(&key) {
        app.move_column_right();
    }
}

fn handle_input(app: &mut App, key: KeyCode, cfg: &Config) {
    let k = &cfg.keys;
    if k.popup_next_field.matches(&key) {
        app.focused_field = match app.focused_field {
            PopupField::Title       => { app.editing_desc = true;  PopupField::Description }
            PopupField::Description => { app.editing_desc = false; PopupField::Tags }
            PopupField::Tags        => { app.editing_desc = false; PopupField::Title }
        };
    } else if k.popup_confirm.matches(&key) {
        if app.focused_field == PopupField::Description {
            let cursor = app.desc_cursor;
            app.desc_buffer.insert(cursor, '\n');
            app.desc_cursor = cursor + 1;
            return;
        }
        if !app.input_buffer.trim().is_empty() {
            let title = app.input_buffer.trim().to_string();
            let desc  = app.desc_buffer.trim_end().to_string();
            let tags  = app.parse_tags();
            match app.input_mode.clone() {
                InputMode::AddingCard => app.add_card(title, desc, tags),
                InputMode::EditingCard { col, card } => {
                    app.columns[col].cards[card].title       = title;
                    app.columns[col].cards[card].description = desc;
                    app.columns[col].cards[card].tags        = tags;
                    app.save();
                }
                _ => {}
            }
        }
        app.input_mode = InputMode::Normal;
        app.reset_popup();
    } else if k.popup_cancel.matches(&key) {
        app.input_mode = InputMode::Normal;
        app.reset_popup();
    } else {
        handle_text_input(app, key);
    }
}

fn handle_column_input(app: &mut App, key: KeyCode, cfg: &Config) {
    let k = &cfg.keys;
    if k.popup_cancel.matches(&key) {
        app.input_mode = InputMode::Normal;
        app.input_buffer.clear();
        app.title_cursor = 0;
    } else if k.popup_confirm.matches(&key) {
        let name = app.input_buffer.trim().to_string();
        if !name.is_empty() {
            match app.input_mode.clone() {
                InputMode::AddingColumn           => app.add_column(name),
                InputMode::RenamingColumn { col } => app.rename_column(col, name),
                _ => {}
            }
        }
        app.input_mode = InputMode::Normal;
        app.input_buffer.clear();
        app.title_cursor = 0;
    } else {
        match key {
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
}

fn handle_delete_confirm(app: &mut App, key: KeyCode, cfg: &Config) {
    let k = &cfg.keys;
    if k.popup_cancel.matches(&key) {
        app.input_mode = InputMode::Normal;
        app.input_buffer.clear();
    } else if k.popup_confirm.matches(&key) {
        if app.input_buffer.trim().eq_ignore_ascii_case("yes") {
            if let InputMode::DeletingColumn { col } = app.input_mode.clone() {
                app.delete_column(col);
            }
        }
        app.input_mode = InputMode::Normal;
        app.input_buffer.clear();
    } else {
        match key {
            KeyCode::Backspace => { app.input_buffer.pop(); }
            KeyCode::Char(c)   => { app.input_buffer.push(c); }
            _ => {}
        }
    }
}

fn handle_text_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Left => match app.focused_field {
            PopupField::Title => { let mut c = app.title_cursor; App::cursor_left(&app.input_buffer.clone(), &mut c); app.title_cursor = c; }
            PopupField::Description => { let mut c = app.desc_cursor; App::cursor_left(&app.desc_buffer.clone(), &mut c); app.desc_cursor = c; }
            PopupField::Tags => { let mut c = app.tags_cursor; App::cursor_left(&app.tags_buffer.clone(), &mut c); app.tags_cursor = c; }
        },
        KeyCode::Right => match app.focused_field {
            PopupField::Title => { let mut c = app.title_cursor; App::cursor_right(&app.input_buffer.clone(), &mut c); app.title_cursor = c; }
            PopupField::Description => { let mut c = app.desc_cursor; App::cursor_right(&app.desc_buffer.clone(), &mut c); app.desc_cursor = c; }
            PopupField::Tags => { let mut c = app.tags_cursor; App::cursor_right(&app.tags_buffer.clone(), &mut c); app.tags_cursor = c; }
        },
        KeyCode::Backspace => match app.focused_field {
            PopupField::Title => { let (mut buf, mut cur) = (app.input_buffer.clone(), app.title_cursor); App::delete_char_before(&mut buf, &mut cur); app.input_buffer = buf; app.title_cursor = cur; }
            PopupField::Description => { let (mut buf, mut cur) = (app.desc_buffer.clone(), app.desc_cursor); App::delete_char_before(&mut buf, &mut cur); app.desc_buffer = buf; app.desc_cursor = cur; }
            PopupField::Tags => { let (mut buf, mut cur) = (app.tags_buffer.clone(), app.tags_cursor); App::delete_char_before(&mut buf, &mut cur); app.tags_buffer = buf; app.tags_cursor = cur; }
        },
        KeyCode::Char(c) => match app.focused_field {
            PopupField::Title => { let (mut buf, mut cur) = (app.input_buffer.clone(), app.title_cursor); App::insert_char(&mut buf, &mut cur, c); app.input_buffer = buf; app.title_cursor = cur; }
            PopupField::Description => { let (mut buf, mut cur) = (app.desc_buffer.clone(), app.desc_cursor); App::insert_char(&mut buf, &mut cur, c); app.desc_buffer = buf; app.desc_cursor = cur; }
            PopupField::Tags => { let (mut buf, mut cur) = (app.tags_buffer.clone(), app.tags_cursor); App::insert_char(&mut buf, &mut cur, c); app.tags_buffer = buf; app.tags_cursor = cur; }
        },
        _ => {}
    }
}
