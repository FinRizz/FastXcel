use fastxcel::columns::{build_column_plan, ColumnPlanError};
use fastxcel::filter::{parse_filter, ComparisonOp, FilterExpr, FilterParseError, FilterValue};
use fastxcel::model::SourceColumnId;
use fastxcel::numeric::{parse_numeric, NumericError};
use polars::prelude::*;

fn schema_with_names(names: &[&str]) -> Schema {
    names
        .iter()
        .map(|name| ((*name).into(), DataType::Int64))
        .collect()
}

#[test]
fn numeric_parser_accepts_signed_decimals_and_exponents() {
    assert_eq!(parse_numeric(" +12.5 "), Ok(12.5));
    assert_eq!(parse_numeric("-1e3"), Ok(-1000.0));
    assert_eq!(parse_numeric(".25"), Ok(0.25));
}

#[test]
fn numeric_parser_rejects_separators_currency_nonfinite_and_overflow() {
    assert_eq!(parse_numeric("1,200"), Err(NumericError::InvalidCharacter));
    assert_eq!(parse_numeric("$120"), Err(NumericError::InvalidCharacter));
    assert_eq!(parse_numeric("NaN"), Err(NumericError::NotFinite));
    assert_eq!(parse_numeric("1e400"), Err(NumericError::Overflow));
}

#[test]
fn filter_parser_honors_precedence_and_full_consumption() {
    let expr = parse_filter("A > 1 | B > 2 & C > 3").unwrap();
    assert_eq!(
        expr,
        FilterExpr::Or(
            Box::new(FilterExpr::Compare {
                column: "A".into(),
                op: ComparisonOp::Gt,
                value: FilterValue::Number(1.0),
            }),
            Box::new(FilterExpr::And(
                Box::new(FilterExpr::Compare {
                    column: "B".into(),
                    op: ComparisonOp::Gt,
                    value: FilterValue::Number(2.0),
                }),
                Box::new(FilterExpr::Compare {
                    column: "C".into(),
                    op: ComparisonOp::Gt,
                    value: FilterValue::Number(3.0),
                }),
            )),
        )
    );

    assert!(matches!(
        parse_filter("A > 1 trailing"),
        Err(FilterParseError::TrailingTokens { .. })
    ));
}

#[test]
fn filter_parser_allows_string_equality_but_not_ordering() {
    let expr = parse_filter("Symbol == TCS").unwrap();
    assert_eq!(
        expr,
        FilterExpr::Compare {
            column: "Symbol".into(),
            op: ComparisonOp::Eq,
            value: FilterValue::Text("TCS".into()),
        }
    );

    assert!(matches!(
        parse_filter("Symbol > TCS"),
        Err(FilterParseError::InvalidComparison { .. })
    ));
}

#[test]
fn column_plan_keeps_duplicate_aliases_distinguishable() {
    let schema = schema_with_names(&["ClsPric", "SttlmPric", "Volume"]);
    let plan = build_column_plan(&schema).unwrap();

    assert_eq!(plan.columns[0].source_column_id, SourceColumnId::new(1));
    assert_eq!(plan.columns[1].source_column_id, SourceColumnId::new(2));
    assert_eq!(plan.columns[0].canonical_name, "Close");
    assert_eq!(plan.columns[1].canonical_name, "Close");
    assert_ne!(plan.columns[0].display_name, plan.columns[1].display_name);
    assert!(plan.columns[1].display_name.starts_with("Close"));
}

#[test]
fn column_plan_rejects_empty_schemas() {
    let schema = Schema::default();
    assert!(matches!(
        build_column_plan(&schema),
        Err(ColumnPlanError::EmptySchema)
    ));
}
