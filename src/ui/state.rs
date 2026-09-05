use crate::model::CellId;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppState {
    pub open_path: Option<PathBuf>,
    pub page_size: usize,
    pub current_offset: usize,
    pub filter_query: String,
    pub last_error: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            open_path: None,
            page_size: 100_000,
            current_offset: 0,
            filter_query: String::new(),
            last_error: None,
        }
    }

    pub fn open_success(&mut self, path: PathBuf) {
        self.open_path = Some(path);
        self.current_offset = 0;
        self.last_error = None;
    }

    pub fn open_failed(&mut self, message: impl Into<String>) {
        self.last_error = Some(message.into());
    }

    pub fn set_page_size(&mut self, page_size: usize) {
        self.page_size = page_size.max(1);
        self.current_offset = 0;
    }

    pub fn prev_page(&mut self) {
        self.current_offset = self.current_offset.saturating_sub(self.page_size);
    }

    pub fn next_page(&mut self) {
        self.current_offset = self.current_offset.saturating_add(self.page_size);
    }

    pub fn set_filter_query(&mut self, query: impl Into<String>) {
        self.filter_query = query.into();
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppCommand {
    OpenSucceeded(PathBuf),
    OpenFailed(String),
    SetPageSize(usize),
    PrevPage,
    NextPage,
    SetFilterQuery(String),
    ClearError,
    InterpretCell { cell_id: CellId, value: Option<f64> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppEvent {
    Opened(PathBuf),
    Failed(String),
    PageMoved(usize),
    FilterChanged(String),
    ErrorCleared,
    InterpretationChanged { cell_id: CellId, value: Option<f64> },
}

impl AppState {
    pub fn apply(&mut self, command: AppCommand) -> Vec<AppEvent> {
        match command {
            AppCommand::OpenSucceeded(path) => {
                self.open_success(path.clone());
                vec![AppEvent::Opened(path)]
            }
            AppCommand::OpenFailed(message) => {
                self.open_failed(message.clone());
                vec![AppEvent::Failed(message)]
            }
            AppCommand::SetPageSize(page_size) => {
                self.set_page_size(page_size);
                vec![AppEvent::PageMoved(self.current_offset)]
            }
            AppCommand::PrevPage => {
                self.prev_page();
                vec![AppEvent::PageMoved(self.current_offset)]
            }
            AppCommand::NextPage => {
                self.next_page();
                vec![AppEvent::PageMoved(self.current_offset)]
            }
            AppCommand::SetFilterQuery(query) => {
                self.set_filter_query(query.clone());
                vec![AppEvent::FilterChanged(query)]
            }
            AppCommand::ClearError => {
                self.clear_error();
                vec![AppEvent::ErrorCleared]
            }
            AppCommand::InterpretCell { cell_id, value } => {
                vec![AppEvent::InterpretationChanged { cell_id, value }]
            }
        }
    }
}
