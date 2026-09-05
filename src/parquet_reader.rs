use polars::prelude::*;
use std::path::Path;

pub fn open_schema(path: &Path) -> PolarsResult<Schema> {
    let mut lf = LazyFrame::scan_parquet(path, ScanArgsParquet::default())?;
    Ok(lf.collect_schema()?.as_ref().clone())
}
