use fastxcel::data::DataEngine;
use fastxcel::ui::table::cell_str;
use polars::prelude::*;

#[test]
fn parquet_preserves_temporal_nested_and_numeric_columns_with_filters_and_paging() {
    let timestamp = Series::new("event_time".into(), &[Some(0_i64), None, Some(-1)])
        .i64()
        .unwrap()
        .clone()
        .into_datetime(TimeUnit::Nanoseconds, Some("+05:30".into()))
        .into_series();
    let date = Series::new("day".into(), &[0_i32, 1, 2])
        .cast(&DataType::Date)
        .unwrap();
    let decimal = Series::new("amount".into(), &[1.25_f64, 2.5, 3.75])
        .cast(&DataType::Decimal(Some(10), Some(2)))
        .unwrap();
    let small = Series::new("small".into(), &[1_i32, 2, 3])
        .cast(&DataType::Int8)
        .unwrap();
    let list = Series::new(
        "items".into(),
        &[
            Series::new("".into(), &[1_i64, 2]),
            Series::new("".into(), &[3_i64]),
            Series::new("".into(), &[4_i64]),
        ],
    );
    let mut df = DataFrame::new(vec![
        timestamp,
        date,
        decimal,
        small,
        list,
        Series::new("value".into(), &[1_i64, 2, 3]),
        Series::new("__source_row_id".into(), &["a", "b", "c"]),
        Series::new("timestamp_note".into(), &["keep", "all", "columns"]),
    ])
    .unwrap();
    let path = std::env::temp_dir().join(format!("fastxcel-schema-{}.parquet", std::process::id()));
    ParquetWriter::new(std::fs::File::create(&path).unwrap())
        .finish(&mut df)
        .unwrap();
    let mut engine = DataEngine::new();
    engine.open_path(&path).unwrap();
    let (page, more) = engine.fetch_page(0, 2, "").unwrap();
    assert!(more);
    assert_eq!(page.width(), df.width() + 1);
    for (actual, source) in page.get_columns().iter().skip(1).zip(df.get_columns()) {
        assert_eq!(actual.dtype(), source.dtype());
    }
    assert_eq!(
        cell_str(page.column("event_time").unwrap(), 0),
        "1970-01-01T05:30:00+05:30"
    );
    assert_eq!(cell_str(page.column("event_time").unwrap(), 1), "");
    let (filtered, more) = engine.fetch_page(0, 2, "value > 1").unwrap();
    assert_eq!(filtered.height(), 2);
    assert!(!more);
    assert_eq!(
        filtered.column("value").unwrap().i64().unwrap().get(0),
        Some(2)
    );
    assert_eq!(
        cell_str(filtered.column("event_time").unwrap(), 1),
        "1970-01-01T05:29:59.999999999+05:30"
    );
    let (last, more) = engine.fetch_page(2, 1, "").unwrap();
    assert_eq!(last.height(), 1);
    assert!(!more);
    assert!(engine.fetch_page(0, 0, "").is_err());
    drop(engine);
    std::fs::remove_file(path).unwrap();
}

#[test]
#[ignore = "set FASTXCEL_TEST_PARQUET to a local Parquet file"]
fn local_parquet_preserves_every_source_column() {
    let path = std::env::var_os("FASTXCEL_TEST_PARQUET").expect("set FASTXCEL_TEST_PARQUET");
    let mut engine = DataEngine::new();
    engine.open_path(std::path::Path::new(&path)).unwrap();
    let schema = engine.current_schema().unwrap().clone();
    let (page, _) = engine.fetch_page(0, 100, "").unwrap();
    assert_eq!(
        page.width(),
        schema.len() + 1,
        "source columns must not disappear"
    );
    for (column, (_, dtype)) in page.get_columns().iter().skip(1).zip(schema.iter()) {
        assert_eq!(column.dtype(), dtype);
    }
    assert!(page.height() > 0);
}
