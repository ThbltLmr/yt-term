use crate::tui::search::SearchResult;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Search,
    Results,
    Playing,
}

pub struct App {
    pub mode: AppMode,
    pub search_input: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub should_quit: bool,
    pub playing_title: Option<String>,
    pub playing_url: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Search,
            search_input: String::new(),
            results: Vec::new(),
            selected_index: 0,
            should_quit: false,
            playing_title: None,
            playing_url: None,
        }
    }

    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = self.selected_index.checked_sub(1).unwrap_or(self.results.len() - 1);
        }
    }

    pub fn get_selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected_index)
    }
}
