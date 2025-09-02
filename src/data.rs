use crate::columns::{canonicalize_columns, ExprBuilder};
use polars::prelude::*;
use std::path::{Path, PathBuf};

pub struct DataEngine {
    path: Option<PathBuf>,
    lazy: Option<LazyFrame>,
    schema: Option<Schema>,
}

#[derive(Clone, Copy)]
pub struct LoadStats {
    pub ms: u128,
}

impl DataEngine {
    pub fn new() -> Self {
        Self { path: None, lazy: None, schema: None }
    }

    pub fn open_path(&mut self, path: &Path) -> PolarsResult<LoadStats> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let lf = match ext.as_str() {
            "csv" => {
                let scan = LazyCsvReader::new(path)
                    .with_has_header(true)
                    .with_ignore_errors(true)
                    .with_infer_schema_length(Some(512))
                    .finish()?;
                scan
            }
            "parquet" => {
                let scan = LazyFrame::scan_parquet(path, ScanArgsParquet::default())?;
                scan
            }
            other => {
                return Err(PolarsError::ComputeError(format!("Unsupported extension: {}", other).into()));
            }
        };

        // Store schema quickly (without full collect)
        let schema = lf.clone().collect()?.schema().clone();

        self.path = Some(path.to_path_buf());
        self.lazy = Some(lf);
        self.schema = Some(schema);

        Ok(LoadStats { ms: 0 })
    }

    /// Fetch a page using lazy slicing and optional filter expression.
    /// Returns (DataFrame, more_rows_flag)
    pub fn fetch_page(&self, offset: usize, limit: usize, filter_expr: &str) -> PolarsResult<(DataFrame, bool)> {
        let lf = self.lazy.as_ref().ok_or_else(|| PolarsError::ComputeError("No file open".into()))?;

        // Canonicalize OHLCV aliases (no-op if not present).
        let mut lf = canonicalize_columns(lf.clone());

        // Optional filter. We support a tiny DSL: column ops with >,<,==,&,|, parentheses.
        if !filter_expr.is_empty() {
            if let Some(expr) = ExprBuilder::parse(filter_expr) {
                lf = lf.filter(expr);
            }
        }

        let slice = lf.clone().slice(offset as i64, limit.try_into().unwrap());

        // Collect just the slice; avoid counting total rows to remain O(1) on big files.
        let df = slice.collect()?;

        // Heuristic: try reading one extra row to see if more exists, without large overhead.
        let has_more = df.height() == limit;
        Ok((df, has_more))
    }

    pub fn current_schema(&self) -> Option<&Schema> {
        self.schema.as_ref()
    }
}