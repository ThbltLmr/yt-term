pub mod app;
pub mod search;
pub mod terminal;
pub mod ui;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

use app::{App, AppMode};
use search::search_youtube;

pub fn run<F>(start_playback: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<(), Box<dyn std::error::Error>>,
{
    let mut terminal = terminal::init()?;
    let mut app = App::new();

    let result = run_app(&mut terminal, &mut app, start_playback);

    terminal::restore()?;
    result
}

fn run_app<F>(
    terminal: &mut terminal::Tui,
    app: &mut App,
    start_playback: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<(), Box<dyn std::error::Error>>,
{
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match &app.mode {
                    AppMode::Search => handle_search_mode(app, key.code),
                    AppMode::Results => handle_results_mode(app, key.code, &start_playback)?,
                    AppMode::Playing => handle_playing_mode(app, key.code),
                }
            }
        }
    }

    Ok(())
}

fn handle_search_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') if app.search_input.is_empty() => {
            app.should_quit = true;
        }
        KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
        }
        KeyCode::Backspace => {
            app.search_input.pop();
        }
        KeyCode::Enter => {
            if !app.search_input.is_empty() {
                if let Ok(results) = search_youtube(&app.search_input, 10) {
                    app.results = results;
                    app.selected_index = 0;
                    if !app.results.is_empty() {
                        app.mode = AppMode::Results;
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_results_mode<F>(
    app: &mut App,
    key: KeyCode,
    start_playback: &F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<(), Box<dyn std::error::Error>>,
{
    match key {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Esc => {
            app.mode = AppMode::Search;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_previous();
        }
        KeyCode::Enter => {
            if let Some(result) = app.get_selected_result().cloned() {
                app.playing_title = Some(result.title.clone());
                app.playing_url = Some(result.url.clone());
                app.mode = AppMode::Playing;

                start_playback(&result.url)?;

                app.mode = AppMode::Results;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_playing_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Results;
        }
        _ => {}
    }
}
