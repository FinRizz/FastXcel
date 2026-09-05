use crate::columns::canonicalize_columns;
use crate::filter::parse_filter;
use crate::model::{
    CellId, FileGeneration, FilterRevision, OverrideRevision, PageCache, PageCacheKey,
    SemanticRevision, SessionOverrides,
};
use crate::parquet_reader::open_schema as open_parquet_schema;
use polars::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub struct DataEngine {
    path: Option<PathBuf>,
    lazy: Option<LazyFrame>,
    schema: Option<Schema>,
    row_index_offset: u32,
    file_generation: FileGeneration,
    filter_revision: FilterRevision,
    semantic_revision: SemanticRevision,
    override_revision: OverrideRevision,
    page_cache: PageCache<(DataFrame, bool)>,
    session_overrides: SessionOverrides,
}

#[derive(Clone, Copy)]
pub struct LoadStats {
    pub ms: u128,
}

impl DataEngine {
    pub fn new() -> Self {
        Self {
            path: None,
            lazy: None,
            schema: None,
            row_index_offset: 0,
            file_generation: FileGeneration::default(),
            filter_revision: FilterRevision::default(),
            semantic_revision: SemanticRevision::default(),
            override_revision: OverrideRevision::default(),
            page_cache: PageCache::default(),
            session_overrides: SessionOverrides::default(),
        }
    }

    pub fn open_path(&mut self, path: &Path) -> PolarsResult<LoadStats> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let (lf, schema, row_index_offset) = match ext.as_str() {
            "csv" => {
                let lf = LazyCsvReader::new(path)
                    .with_has_header(true)
                    .with_ignore_errors(true)
                    .with_infer_schema_length(Some(512))
                    .finish()?;
                let mut schema_lf = lf.clone();
                let schema = schema_lf.collect_schema()?.as_ref().clone();
                (lf, schema, 1)
            }
            "parquet" => {
                let lf = LazyFrame::scan_parquet(path, ScanArgsParquet::default())?;
                let schema = open_parquet_schema(path)?;
                (lf, schema, 0)
            }
            other => {
                return Err(PolarsError::ComputeError(
                    format!("Unsupported extension: {}", other).into(),
                ));
            }
        };

        self.path = Some(path.to_path_buf());
        self.lazy = Some(lf);
        self.schema = Some(schema);
        self.row_index_offset = row_index_offset;
        self.file_generation = self.file_generation.next();
        self.filter_revision = FilterRevision::default();
        self.semantic_revision = SemanticRevision::default();
        self.override_revision = OverrideRevision::default();
        self.page_cache.clear();
        self.session_overrides.clear();

        Ok(LoadStats { ms: 0 })
    }

    /// Fetch a page using lazy slicing and optional filter expression.
    /// Returns (DataFrame, more_rows_flag)
    pub fn fetch_page(
        &mut self,
        offset: usize,
        limit: usize,
        filter_expr: &str,
    ) -> PolarsResult<(DataFrame, bool)> {
        let lf = self
            .lazy
            .as_ref()
            .ok_or_else(|| PolarsError::ComputeError("No file open".into()))?;

        let lf = lf
            .clone()
            .with_row_index("__source_row_id", Some(self.row_index_offset));
        let original_lf = lf.clone();
        // Canonicalize OHLCV aliases (no-op if not present).
        let mut lf = canonicalize_columns(lf)?;
        let cache_key = self.page_cache_key(offset, limit, filter_expr);
        if let Some((df, has_more)) = self.page_cache.get(&cache_key) {
            return Ok((df.clone(), *has_more));
        }

        // Optional filter. We support a tiny DSL: column ops with >,<,==,&,|, parentheses.
        if !filter_expr.is_empty() {
            let expr = parse_filter(filter_expr).map_err(|error| {
                PolarsError::ComputeError(format!("invalid filter expression: {error}").into())
            })?;
            lf = lf.filter(expr.to_polars_expr());
        }

        let df = match lf
            .clone()
            .slice(offset as i64, limit.try_into().unwrap())
            .collect()
        {
            Ok(df) => df,
            Err(error) if is_timestamp_materialization_error(&error) => {
                let fallback_columns = fallback_columns(self.schema.as_ref());
                if fallback_columns.is_empty() {
                    return Err(error);
                }

                let fallback_exprs = std::iter::once(col("__source_row_id"))
                    .chain(fallback_columns.iter().map(|name| col(name.as_str())))
                    .collect::<Vec<_>>();
                let fallback_lf = canonicalize_columns(original_lf.select(fallback_exprs))?;
                fallback_lf
                    .slice(offset as i64, limit.try_into().unwrap())
                    .collect()?
            }
            Err(error) => return Err(error),
        };

        // Heuristic: try reading one extra row to see if more exists, without large overhead.
        let has_more = df.height() == limit;
        self.page_cache.insert(cache_key, (df.clone(), has_more));
        Ok((df, has_more))
    }

    pub fn current_schema(&self) -> Option<&Schema> {
        self.schema.as_ref()
    }

    pub fn page_cache_key(&self, offset: usize, limit: usize, filter_expr: &str) -> PageCacheKey {
        PageCacheKey::new(
            self.file_generation,
            offset,
            limit,
            FilterRevision::new(filter_signature(filter_expr)),
            self.semantic_revision,
            self.override_revision,
        )
    }

    pub fn file_generation(&self) -> FileGeneration {
        self.file_generation
    }

    pub fn filter_revision(&self) -> FilterRevision {
        self.filter_revision
    }

    pub fn semantic_revision(&self) -> SemanticRevision {
        self.semantic_revision
    }

    pub fn override_revision(&self) -> OverrideRevision {
        self.override_revision
    }

    pub fn bump_filter_revision(&mut self) {
        self.filter_revision = self.filter_revision.next();
        self.page_cache.clear();
    }

    pub fn bump_semantic_revision(&mut self) {
        self.semantic_revision = self.semantic_revision.next();
        self.page_cache.clear();
    }

    pub fn bump_override_revision(&mut self) {
        self.override_revision = self.override_revision.next();
        self.page_cache.clear();
    }

    pub fn override_for_cell(&self, cell_id: &CellId) -> Option<f64> {
        self.session_overrides.get(cell_id)
    }

    pub fn set_override(&mut self, cell_id: CellId, value: f64) {
        self.session_overrides.set(cell_id, value);
        self.bump_override_revision();
    }

    pub fn remove_override(&mut self, cell_id: &CellId) -> Option<f64> {
        let removed = self.session_overrides.remove(cell_id);
        if removed.is_some() {
            self.bump_override_revision();
        }
        removed
    }
}

impl Default for DataEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn filter_signature(filter_expr: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    filter_expr.hash(&mut hasher);
    hasher.finish()
}

fn fallback_columns(schema: Option<&Schema>) -> Vec<String> {
    schema
        .into_iter()
        .flat_map(|schema| schema.iter_fields())
        .filter_map(|field| {
            let name = field.name().as_str();
            let lower = name.to_ascii_lowercase();
            let problematic_name = lower.contains("timestamp") || lower.contains("datetime");
            let problematic_dtype = matches!(
                field.dtype(),
                DataType::Datetime(_, _) | DataType::Date | DataType::Time
            );

            if problematic_name || problematic_dtype {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

fn is_timestamp_materialization_error(error: &PolarsError) -> bool {
    error
        .to_string()
        .contains("cannot create series from Timestamp(Second, None)")
}
