mod app;
mod ui;

use std::{io, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    while !app.quit {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.quit = true,

                    KeyCode::Left => {
                        if app.selected_column > 0 {
                            app.selected_column -= 1;
                        }
                    }

                    KeyCode::Right => {
                        if app.selected_column + 1 < app.columns.len() {
                            app.selected_column += 1;
                        }
                    }

                    KeyCode::Up => {
                        let col = &mut app.columns[app.selected_column];
                        if col.selected > 0 {
                            col.selected -= 1;
                        }
                    }

                    KeyCode::Down => {
                        let col = &mut app.columns[app.selected_column];
                        if col.selected + 1 < col.cards.len() {
                            col.selected += 1;
                        }
                    }

                    _ => {}
                }
            }
        }
    }
    Ok(())
}
