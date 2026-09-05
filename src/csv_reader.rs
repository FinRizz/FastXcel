//! Synchronous, byte-preserving CSV ingestion spike.
//!
//! This module deliberately stops short of the production worker and paging
//! APIs. It proves the parser/recovery boundary that those APIs can build on.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub type SourceRowId = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ByteSpan {
    pub start: u64,
    pub end: u64,
    pub start_line: u64,
}

impl ByteSpan {
    pub fn len(self) -> u64 {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackReason {
    InvalidEncoding,
    ExtraFields,
    InvalidCsv,
    UnclosedQuote,
    RecordTooLarge { limit: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CsvRow {
    Mapped {
        source_row_id: SourceRowId,
        span: ByteSpan,
        fields: Vec<Option<Vec<u8>>>,
    },
    RawFallback {
        source_row_id: SourceRowId,
        span: ByteSpan,
        reason: FallbackReason,
    },
}

impl CsvRow {
    pub fn source_row_id(&self) -> SourceRowId {
        match self {
            Self::Mapped { source_row_id, .. } | Self::RawFallback { source_row_id, .. } => {
                *source_row_id
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ObservedKind {
    Integer,
    Decimal,
    Text,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ColumnObservation {
    pub observed: BTreeSet<ObservedKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsvScanReport {
    pub header: Vec<Vec<u8>>,
    pub rows_seen: u64,
    pub structural_anomalies: u64,
    pub completed: bool,
    pub semantic_columns: Vec<ColumnObservation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanDecision {
    Continue,
    Stop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsvScanOptions {
    /// Maximum bytes owned for a single logical record. Larger records remain
    /// available through a file-backed [`ByteSpan`].
    pub max_record_bytes: usize,
    /// Fixed input buffer; scan memory does not grow with file size.
    pub read_buffer_bytes: usize,
}

impl Default for CsvScanOptions {
    fn default() -> Self {
        Self {
            max_record_bytes: 1024 * 1024,
            read_buffer_bytes: 64 * 1024,
        }
    }
}

#[derive(Debug)]
pub enum CsvScanError {
    Io(io::Error),
    InvalidOptions(&'static str),
    InvalidHeader {
        span: ByteSpan,
        reason: FallbackReason,
    },
}

impl fmt::Display for CsvScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "CSV I/O failed: {error}"),
            Self::InvalidOptions(message) => write!(f, "invalid CSV scan options: {message}"),
            Self::InvalidHeader { span, reason } => write!(
                f,
                "CSV header at bytes {}..{} cannot be mapped: {reason:?}",
                span.start, span.end
            ),
        }
    }
}

impl std::error::Error for CsvScanError {}

impl From<io::Error> for CsvScanError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuoteState {
    Outside { at_field_start: bool },
    Quoted,
    AfterQuote,
}

/// Streams a CSV once, assigning one-based IDs to data records after the
/// header. The callback owns at most one bounded mapped record at a time.
pub fn scan_csv(
    path: &Path,
    options: CsvScanOptions,
    mut on_row: impl FnMut(CsvRow) -> ScanDecision,
) -> Result<CsvScanReport, CsvScanError> {
    if options.max_record_bytes == 0 {
        return Err(CsvScanError::InvalidOptions(
            "max_record_bytes must be greater than zero",
        ));
    }
    if options.read_buffer_bytes == 0 {
        return Err(CsvScanError::InvalidOptions(
            "read_buffer_bytes must be greater than zero",
        ));
    }

    let mut file = File::open(path)?;
    let mut input = vec![0; options.read_buffer_bytes];
    let mut owned = Vec::with_capacity(options.max_record_bytes.min(64 * 1024));
    let mut oversized = false;
    let mut state = QuoteState::Outside {
        at_field_start: true,
    };
    let mut offset = 0_u64;
    let mut record_start = 0_u64;
    let mut physical_line = 1_u64;
    let mut record_start_line = 1_u64;
    let mut header: Option<Vec<Vec<u8>>> = None;
    let mut rows_seen = 0_u64;
    let mut structural_anomalies = 0_u64;
    let mut semantic_columns = Vec::new();
    let mut stopped = false;

    'scan: loop {
        let read = file.read(&mut input)?;
        if read == 0 {
            break;
        }

        for &byte in &input[..read] {
            if !oversized {
                if owned.len() < options.max_record_bytes {
                    owned.push(byte);
                } else {
                    owned.clear();
                    oversized = true;
                }
            }

            let boundary = match state {
                QuoteState::Outside { at_field_start } => match byte {
                    b'"' if at_field_start => {
                        state = QuoteState::Quoted;
                        false
                    }
                    b',' => {
                        state = QuoteState::Outside {
                            at_field_start: true,
                        };
                        false
                    }
                    b'\n' => true,
                    _ => {
                        state = QuoteState::Outside {
                            at_field_start: false,
                        };
                        false
                    }
                },
                QuoteState::Quoted => {
                    if byte == b'"' {
                        state = QuoteState::AfterQuote;
                    }
                    false
                }
                QuoteState::AfterQuote => match byte {
                    b'"' => {
                        state = QuoteState::Quoted;
                        false
                    }
                    b',' => {
                        state = QuoteState::Outside {
                            at_field_start: true,
                        };
                        false
                    }
                    b'\n' => true,
                    _ => {
                        state = QuoteState::Outside {
                            at_field_start: false,
                        };
                        false
                    }
                },
            };

            offset += 1;
            if byte == b'\n' {
                physical_line += 1;
            }

            if boundary {
                let span = ByteSpan {
                    start: record_start,
                    end: offset,
                    start_line: record_start_line,
                };
                if consume_record(
                    span,
                    &owned,
                    oversized,
                    false,
                    options.max_record_bytes,
                    &mut header,
                    &mut rows_seen,
                    &mut structural_anomalies,
                    &mut semantic_columns,
                    &mut on_row,
                )? == ScanDecision::Stop
                {
                    stopped = true;
                    break 'scan;
                }
                owned.clear();
                oversized = false;
                state = QuoteState::Outside {
                    at_field_start: true,
                };
                record_start = offset;
                record_start_line = physical_line;
            }
        }
    }

    if !stopped && record_start < offset {
        let span = ByteSpan {
            start: record_start,
            end: offset,
            start_line: record_start_line,
        };
        let unclosed = state == QuoteState::Quoted;
        if consume_record(
            span,
            &owned,
            oversized,
            unclosed,
            options.max_record_bytes,
            &mut header,
            &mut rows_seen,
            &mut structural_anomalies,
            &mut semantic_columns,
            &mut on_row,
        )? == ScanDecision::Stop
        {
            stopped = true;
        }
    }

    let header = header.unwrap_or_default();
    if semantic_columns.is_empty() {
        semantic_columns.resize_with(header.len(), ColumnObservation::default);
    }
    Ok(CsvScanReport {
        header,
        rows_seen,
        structural_anomalies,
        completed: !stopped,
        semantic_columns,
    })
}

#[allow(clippy::too_many_arguments)]
fn consume_record(
    span: ByteSpan,
    owned: &[u8],
    oversized: bool,
    unclosed: bool,
    limit: usize,
    header: &mut Option<Vec<Vec<u8>>>,
    rows_seen: &mut u64,
    structural_anomalies: &mut u64,
    semantic_columns: &mut Vec<ColumnObservation>,
    on_row: &mut impl FnMut(CsvRow) -> ScanDecision,
) -> Result<ScanDecision, CsvScanError> {
    let forced_reason = if unclosed {
        Some(FallbackReason::UnclosedQuote)
    } else if oversized {
        Some(FallbackReason::RecordTooLarge { limit })
    } else {
        None
    };

    if header.is_none() {
        if let Some(reason) = forced_reason {
            return Err(CsvScanError::InvalidHeader { span, reason });
        }
        let fields = parse_byte_record(owned)
            .map_err(|reason| CsvScanError::InvalidHeader { span, reason })?;
        semantic_columns.resize_with(fields.len(), ColumnObservation::default);
        *header = Some(fields);
        return Ok(ScanDecision::Continue);
    }

    *rows_seen += 1;
    let source_row_id = *rows_seen;
    let expected = header.as_ref().map_or(0, Vec::len);
    let row = if let Some(reason) = forced_reason {
        *structural_anomalies += 1;
        CsvRow::RawFallback {
            source_row_id,
            span,
            reason,
        }
    } else {
        match parse_byte_record(owned) {
            Ok(fields) if fields.len() > expected => {
                *structural_anomalies += 1;
                CsvRow::RawFallback {
                    source_row_id,
                    span,
                    reason: FallbackReason::ExtraFields,
                }
            }
            Ok(fields)
                if fields
                    .iter()
                    .any(|field| std::str::from_utf8(field).is_err()) =>
            {
                *structural_anomalies += 1;
                CsvRow::RawFallback {
                    source_row_id,
                    span,
                    reason: FallbackReason::InvalidEncoding,
                }
            }
            Ok(fields) => {
                let mut mapped: Vec<Option<Vec<u8>>> = fields.into_iter().map(Some).collect();
                mapped.resize(expected, None);
                observe_fields(&mapped, semantic_columns);
                CsvRow::Mapped {
                    source_row_id,
                    span,
                    fields: mapped,
                }
            }
            Err(reason) => {
                *structural_anomalies += 1;
                CsvRow::RawFallback {
                    source_row_id,
                    span,
                    reason,
                }
            }
        }
    };
    Ok(on_row(row))
}

fn parse_byte_record(bytes: &[u8]) -> Result<Vec<Vec<u8>>, FallbackReason> {
    let content = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let content = content.strip_suffix(b"\r").unwrap_or(content);
    if content.is_empty() {
        return Ok(vec![Vec::new()]);
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum ParseState {
        FieldStart,
        Unquoted,
        Quoted,
        AfterQuote,
    }

    let mut fields = Vec::new();
    let mut field = Vec::new();
    let mut state = ParseState::FieldStart;
    let mut index = 0;

    while index < content.len() {
        let byte = content[index];
        match state {
            ParseState::FieldStart => match byte {
                b',' => {
                    fields.push(std::mem::take(&mut field));
                }
                b'"' => {
                    state = ParseState::Quoted;
                }
                _ => {
                    field.push(byte);
                    state = ParseState::Unquoted;
                }
            },
            ParseState::Unquoted => match byte {
                b',' => {
                    fields.push(std::mem::take(&mut field));
                    state = ParseState::FieldStart;
                }
                b'"' => return Err(FallbackReason::InvalidCsv),
                _ => field.push(byte),
            },
            ParseState::Quoted => match byte {
                b'"' => {
                    state = ParseState::AfterQuote;
                }
                _ => field.push(byte),
            },
            ParseState::AfterQuote => match byte {
                b'"' => {
                    field.push(b'"');
                    state = ParseState::Quoted;
                }
                b',' => {
                    fields.push(std::mem::take(&mut field));
                    state = ParseState::FieldStart;
                }
                _ => return Err(FallbackReason::InvalidCsv),
            },
        }
        index += 1;
    }

    match state {
        ParseState::Quoted => Err(FallbackReason::InvalidCsv),
        _ => {
            fields.push(field);
            Ok(fields)
        }
    }
}

fn observe_fields(fields: &[Option<Vec<u8>>], columns: &mut [ColumnObservation]) {
    for (field, column) in fields.iter().zip(columns) {
        let Some(field) = field else { continue };
        let Ok(value) = std::str::from_utf8(field) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let kind = if value.parse::<i64>().is_ok() {
            ObservedKind::Integer
        } else if value.parse::<f64>().is_ok_and(f64::is_finite) {
            ObservedKind::Decimal
        } else {
            ObservedKind::Text
        };
        column.observed.insert(kind);
    }
}

/// Reads a fallback span without allocating the whole record. Callers choose
/// the maximum chunk presented to `visit`.
pub fn visit_span_chunks(
    path: &Path,
    span: ByteSpan,
    chunk_bytes: usize,
    mut visit: impl FnMut(&[u8]),
) -> io::Result<()> {
    if chunk_bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "chunk_bytes must be greater than zero",
        ));
    }
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(span.start))?;
    let mut remaining = span.len();
    let mut buffer = vec![0; chunk_bytes];
    while remaining > 0 {
        let wanted = usize::try_from(remaining.min(chunk_bytes as u64)).unwrap_or(chunk_bytes);
        let read = file.read(&mut buffer[..wanted])?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "fallback span extends beyond end of file",
            ));
        }
        visit(&buffer[..read]);
        remaining -= read as u64;
    }
    Ok(())
}
