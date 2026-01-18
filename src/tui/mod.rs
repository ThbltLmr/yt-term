pub mod app;
pub mod search;
pub mod terminal;
pub mod ui;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

use crate::PlaybackHandle;
use app::{App, AppMode};
use search::search_youtube;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = terminal::init()?;
    let mut app = App::new();
    let mut playback: Option<PlaybackHandle> = None;

    let result = run_app(&mut terminal, &mut app, &mut playback);

    // Clean up any running playback
    if let Some(handle) = playback {
        handle.cancel();
        handle.join();
    }

    terminal::restore()?;
    result
}

fn run_app(
    terminal: &mut terminal::Tui,
    app: &mut App,
    playback: &mut Option<PlaybackHandle>,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, app))?;

        // Check if playback finished naturally
        if let Some(ref handle) = playback {
            if handle.is_finished() {
                playback.take().unwrap().join();
                app.mode = AppMode::Results;
            }
        }

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle Ctrl+C globally
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if matches!(app.mode, AppMode::Playing) {
                        stop_playback(app, playback);
                    } else {
                        app.should_quit = true;
                    }
                    continue;
                }

                match &app.mode {
                    AppMode::Search => handle_search_mode(app, key.code),
                    AppMode::Results => handle_results_mode(app, key.code, playback),
                    AppMode::Playing => handle_playing_mode(app, key.code, playback),
                }
            }
        }
    }

    Ok(())
}

fn stop_playback(app: &mut App, playback: &mut Option<PlaybackHandle>) {
    if let Some(handle) = playback.take() {
        handle.cancel();
        handle.join();
    }
    app.mode = AppMode::Results;
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

fn handle_results_mode(app: &mut App, key: KeyCode, playback: &mut Option<PlaybackHandle>) {
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

                // Start playback asynchronously
                *playback = Some(crate::start_playback_async(&result.url, false, None));
            }
        }
        _ => {}
    }
}

fn handle_playing_mode(app: &mut App, key: KeyCode, playback: &mut Option<PlaybackHandle>) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            stop_playback(app, playback);
        }
        _ => {}
    }
}
