use crate::filter::parse_filter;
use crate::model::SourceColumnId;
use polars::prelude::*;
use std::collections::HashMap;

/// Map many real-world aliases to canonical names: Open, High, Low, Close, Volume.
pub fn alias_map() -> HashMap<&'static str, &'static str> {
    let pairs: [(&str, &str); 31] = [
        ("open", "Open"),
        ("opn", "Open"),
        ("op", "Open"),
        ("opnpric", "Open"),
        ("opnprc", "Open"),
        ("high", "High"),
        ("hgh", "High"),
        ("low", "Low"),
        ("lw", "Low"),
        ("close", "Close"),
        ("cls", "Close"),
        ("last", "Close"),
        ("closeprice", "Close"),
        ("volume", "Volume"),
        ("vol", "Volume"),
        ("qty", "Volume"),
        ("totaltradedqty", "Volume"),
        ("hghpric", "High"),
        ("lwpric", "Low"),
        ("clspric", "Close"),
        ("sttlmpric", "Close"),
        ("ttltrfval", "Volume"),
        ("opnintrst", "OpenInterest"),
        ("chnginopnintrst", "ChangeInOpenInterest"),
        ("tckrsymb", "Symbol"),
        ("fininstrmactlxprydt", "Expiry"),
        ("traddt", "TradeDate"),
        ("date", "Date"),
        ("timestamp", "Timestamp"),
        ("time", "Time"),
        ("symbol", "Symbol"),
    ];
    pairs.into_iter().collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnPlanColumn {
    pub source_column_id: SourceColumnId,
    pub source_name: String,
    pub canonical_name: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnPlan {
    pub columns: Vec<ColumnPlanColumn>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColumnPlanError {
    EmptySchema,
}

impl std::fmt::Display for ColumnPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySchema => write!(f, "schema has no columns"),
        }
    }
}

impl std::error::Error for ColumnPlanError {}

impl From<ColumnPlanError> for PolarsError {
    fn from(value: ColumnPlanError) -> Self {
        PolarsError::ComputeError(value.to_string().into())
    }
}

pub fn build_column_plan(schema: &Schema) -> Result<ColumnPlan, ColumnPlanError> {
    if schema.is_empty() {
        return Err(ColumnPlanError::EmptySchema);
    }

    let aliases = alias_map();
    let mut columns = Vec::with_capacity(schema.len());
    let mut display_counts: HashMap<String, usize> = HashMap::new();

    for (index, field) in schema.iter_fields().enumerate() {
        let source_name = field.name().to_string();
        let key = normalize_name(&source_name);
        let canonical_name = aliases
            .get(key.as_str())
            .copied()
            .unwrap_or(source_name.as_str());
        let canonical_name = canonical_name.to_string();
        let display_name = assign_display_name(
            &mut display_counts,
            canonical_name.as_str(),
            &source_name,
            SourceColumnId::new((index + 1) as u64),
        );

        columns.push(ColumnPlanColumn {
            source_column_id: SourceColumnId::new((index + 1) as u64),
            source_name,
            canonical_name,
            display_name,
        });
    }

    Ok(ColumnPlan { columns })
}

pub fn canonicalize_columns(lf: LazyFrame) -> PolarsResult<LazyFrame> {
    let mut lf = lf;
    let schema = lf.collect_schema()?;
    let plan = build_column_plan(schema.as_ref())?;

    let exprs: Vec<Expr> = plan
        .columns
        .iter()
        .map(|column| col(&column.source_name).alias(&column.display_name))
        .collect();

    Ok(lf.select(exprs))
}

fn normalize_name(name: &str) -> String {
    name.to_lowercase().replace([' ', '-', '_'], "")
}

fn assign_display_name(
    counts: &mut HashMap<String, usize>,
    canonical_name: &str,
    source_name: &str,
    source_column_id: SourceColumnId,
) -> String {
    let count = counts.entry(canonical_name.to_string()).or_default();
    *count += 1;
    if *count == 1 {
        canonical_name.to_string()
    } else {
        format!(
            "{canonical_name} #{} ({source_name}:{source_column_id})",
            *count
        )
    }
}

/// Compatibility shim for the current lazy frame path.
///
/// U3 establishes the new filter AST in `crate::filter`; this wrapper keeps the
/// existing data path working until later units switch it over.
pub struct ExprBuilder;

impl ExprBuilder {
    pub fn parse(input: &str) -> Option<Expr> {
        parse_filter(input).ok().map(|expr| expr.to_polars_expr())
    }
}
