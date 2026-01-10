use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::{App, AppMode};
use crate::tui::search::SearchResult;

const VIDEO_ROWS: u16 = 25;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(VIDEO_ROWS), Constraint::Min(5)])
        .split(f.area());

    render_video_area(f, app, chunks[0]);
    render_bottom_area(f, app, chunks[1]);
}

fn render_video_area(f: &mut Frame, app: &App, area: Rect) {
    let block = if matches!(app.mode, AppMode::Playing) {
        Block::default().borders(Borders::NONE)
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title("Video")
    };
    f.render_widget(block, area);
}

fn render_bottom_area(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    render_search_bar(f, app, chunks[0]);
    render_content_area(f, app, chunks[1]);
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let style = if matches!(app.mode, AppMode::Search) {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let search_input = Paragraph::new(app.search_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title("Search"),
    );
    f.render_widget(search_input, area);

    if matches!(app.mode, AppMode::Search) {
        f.set_cursor_position((area.x + app.search_input.len() as u16 + 1, area.y + 1));
    }
}

fn render_content_area(f: &mut Frame, app: &App, area: Rect) {
    match &app.mode {
        AppMode::Search => {
            let help = Paragraph::new("Type to search, Enter to submit, q to quit")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(help, area);
        }
        AppMode::Results => {
            let items: Vec<ListItem> = app
                .results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let style = if i == app.selected_index {
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format_result(r)).style(style)
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Results ({} found)", app.results.len())),
            );
            f.render_widget(list, area);
        }
        AppMode::Playing => {
            let title = app.playing_title.as_deref().unwrap_or("Unknown");
            let status = Paragraph::new(format!("Playing: {}\n\nPress Esc to stop", title))
                .block(Block::default().borders(Borders::ALL).title("Now Playing"));
            f.render_widget(status, area);
        }
    }
}

fn format_result(result: &SearchResult) -> String {
    let duration = result
        .duration
        .map(|d| {
            let mins = (d as u32) / 60;
            let secs = (d as u32) % 60;
            format!(" [{}:{:02}]", mins, secs)
        })
        .unwrap_or_default();

    let channel = result
        .channel
        .as_ref()
        .map(|c| format!(" - {}", c))
        .unwrap_or_default();

    format!("{}{}{}", result.title, channel, duration)
}
