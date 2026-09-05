use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceRowId(u64);

impl SourceRowId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for SourceRowId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for SourceRowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceColumnId(u64);

impl SourceColumnId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for SourceColumnId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for SourceColumnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellId {
    pub row: SourceRowId,
    pub column: SourceColumnId,
}

impl CellId {
    pub fn new(row: SourceRowId, column: SourceColumnId) -> Self {
        Self { row, column }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticRevision(u64);

impl SemanticRevision {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl Default for SemanticRevision {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OverrideRevision(u64);

impl OverrideRevision {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl Default for OverrideRevision {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileGeneration(u64);

impl FileGeneration {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl Default for FileGeneration {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FilterRevision(u64);

impl FilterRevision {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl Default for FilterRevision {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticLifecycle {
    Scanning,
    Partial,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostics {
    pub lifecycle: DiagnosticLifecycle,
    pub structural_anomalies: u64,
    pub semantic_mismatches: u64,
    pub message: Option<String>,
}

impl Diagnostics {
    pub fn scanning() -> Self {
        Self {
            lifecycle: DiagnosticLifecycle::Scanning,
            structural_anomalies: 0,
            semantic_mismatches: 0,
            message: None,
        }
    }

    pub fn partial(structural_anomalies: u64) -> Self {
        Self {
            lifecycle: DiagnosticLifecycle::Partial,
            structural_anomalies,
            semantic_mismatches: 0,
            message: None,
        }
    }

    pub fn complete() -> Self {
        Self {
            lifecycle: DiagnosticLifecycle::Complete,
            structural_anomalies: 0,
            semantic_mismatches: 0,
            message: None,
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            lifecycle: DiagnosticLifecycle::Failed,
            structural_anomalies: 0,
            semantic_mismatches: 0,
            message: Some(message.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

impl ByteRange {
    pub fn len(self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PageCell {
    Missing,
    Text(String),
    Number(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PageRow {
    Mapped {
        source_row_id: SourceRowId,
        cells: Vec<PageCell>,
    },
    RawFallback {
        source_row_id: SourceRowId,
        byte_range: Option<ByteRange>,
        reason: String,
    },
}

impl PageRow {
    pub fn source_row_id(&self) -> SourceRowId {
        match self {
            Self::Mapped { source_row_id, .. } | Self::RawFallback { source_row_id, .. } => {
                *source_row_id
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageColumn {
    pub source_column_id: SourceColumnId,
    pub source_name: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub columns: Vec<PageColumn>,
    pub rows: Vec<PageRow>,
    pub has_more: bool,
    pub diagnostics: Diagnostics,
    pub semantic_revision: SemanticRevision,
    pub override_revision: OverrideRevision,
}

impl Page {
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            has_more: false,
            diagnostics: Diagnostics::complete(),
            semantic_revision: SemanticRevision::default(),
            override_revision: OverrideRevision::default(),
        }
    }

    pub fn with_rows(rows: Vec<PageRow>) -> Self {
        Self {
            rows,
            ..Self::empty()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PageCacheKey {
    pub file_generation: FileGeneration,
    pub offset: usize,
    pub limit: usize,
    pub filter_revision: FilterRevision,
    pub semantic_revision: SemanticRevision,
    pub override_revision: OverrideRevision,
}

impl PageCacheKey {
    pub fn new(
        file_generation: FileGeneration,
        offset: usize,
        limit: usize,
        filter_revision: FilterRevision,
        semantic_revision: SemanticRevision,
        override_revision: OverrideRevision,
    ) -> Self {
        Self {
            file_generation,
            offset,
            limit,
            filter_revision,
            semantic_revision,
            override_revision,
        }
    }
}

pub struct PageCache<V> {
    entries: HashMap<PageCacheKey, V>,
}

impl<V> PageCache<V> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, key: &PageCacheKey) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: PageCacheKey, value: V) {
        self.entries.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<V> Default for PageCache<V> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SessionOverrides {
    entries: HashMap<CellId, f64>,
}

impl SessionOverrides {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, cell_id: &CellId) -> Option<f64> {
        self.entries.get(cell_id).copied()
    }

    pub fn set(&mut self, cell_id: CellId, value: f64) {
        self.entries.insert(cell_id, value);
    }

    pub fn remove(&mut self, cell_id: &CellId) -> Option<f64> {
        self.entries.remove(cell_id)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for SessionOverrides {
    fn default() -> Self {
        Self::new()
    }
}
