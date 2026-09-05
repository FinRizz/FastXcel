use fastxcel::csv_reader::{
    scan_csv, visit_span_chunks, CsvRow, CsvScanOptions, FallbackReason, ObservedKind, ScanDecision,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/csv")
        .join(name)
}

fn temp_csv(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "fastxcel-{name}-{}-{}.csv",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn empty_and_header_only_files_have_no_data_rows() {
    for bytes in [b"".as_slice(), b"a,b\n".as_slice()] {
        let path = temp_csv("empty");
        fs::write(&path, bytes).unwrap();
        let report =
            scan_csv(&path, CsvScanOptions::default(), |_| ScanDecision::Continue).unwrap();
        assert_eq!(report.rows_seen, 0);
        assert!(report.completed);
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn flexible_rows_preserve_missing_and_fallback_uncertain_mappings() {
    let mut rows = Vec::new();
    let report = scan_csv(
        &fixture("flexible_rows.csv"),
        CsvScanOptions::default(),
        |row| {
            rows.push(row);
            ScanDecision::Continue
        },
    )
    .unwrap();

    assert_eq!(rows.len(), 5);
    assert!(
        matches!(&rows[1], CsvRow::Mapped { source_row_id: 2, fields, .. }
        if fields[2].is_none())
    );
    assert!(matches!(
        &rows[2],
        CsvRow::RawFallback {
            source_row_id: 3,
            reason: FallbackReason::ExtraFields,
            ..
        }
    ));
    assert!(
        matches!(&rows[3], CsvRow::Mapped { source_row_id: 4, fields, .. }
        if fields[0].as_deref() == Some(b"".as_slice()) && fields[1].is_none())
    );
    assert!(
        matches!(&rows[4], CsvRow::Mapped { source_row_id: 5, fields, .. }
        if fields[1].as_deref() == Some(b"".as_slice()))
    );
    assert_eq!(report.structural_anomalies, 1);
}

#[test]
fn embedded_newline_is_one_record_with_a_stable_byte_span() {
    let mut rows = Vec::new();
    scan_csv(
        &fixture("embedded_newline.csv"),
        CsvScanOptions::default(),
        |row| {
            rows.push(row);
            ScanDecision::Continue
        },
    )
    .unwrap();

    assert_eq!(rows.len(), 2);
    assert!(
        matches!(&rows[0], CsvRow::Mapped { source_row_id: 1, fields, span }
        if fields[1].as_deref() == Some(b"first line\nsecond line".as_slice())
            && span.start_line == 2
            && span.start == 12
            && span.end == 41)
    );
}

#[test]
fn quoted_commas_and_escaped_quotes_are_preserved() {
    let path = temp_csv("quoted-fields");
    fs::write(
        &path,
        b"name,note\nABC,\"a,b\"\nDEF,\"He said \"\"hi\"\"\"\n",
    )
    .unwrap();

    let mut rows = Vec::new();
    scan_csv(&path, CsvScanOptions::default(), |row| {
        rows.push(row);
        ScanDecision::Continue
    })
    .unwrap();

    assert_eq!(rows.len(), 2);
    assert!(matches!(&rows[0], CsvRow::Mapped { fields, .. }
        if fields[1].as_deref() == Some(b"a,b".as_slice())));
    assert!(matches!(&rows[1], CsvRow::Mapped { fields, .. }
        if fields[1].as_deref() == Some(b"He said \"hi\"".as_slice())));
    fs::remove_file(path).unwrap();
}

#[test]
fn invalid_utf8_becomes_a_positioned_raw_fallback() {
    let path = temp_csv("invalid-utf8");
    fs::write(&path, b"symbol,price\nABC,10\nBAD,\xff\n").unwrap();
    let mut rows = Vec::new();
    scan_csv(&path, CsvScanOptions::default(), |row| {
        rows.push(row);
        ScanDecision::Continue
    })
    .unwrap();

    assert!(matches!(&rows[1], CsvRow::RawFallback {
        source_row_id: 2,
        reason: FallbackReason::InvalidEncoding,
        span,
    } if span.start_line == 3 && span.start == 20 && span.end == 26));
    fs::remove_file(path).unwrap();
}

#[test]
fn unclosed_quote_preserves_the_remaining_file_as_one_span() {
    let path = fixture("unclosed_quote.csv");
    let mut rows = Vec::new();
    scan_csv(&path, CsvScanOptions::default(), |row| {
        rows.push(row);
        ScanDecision::Continue
    })
    .unwrap();

    let (span, source_row_id) = match rows.last().unwrap() {
        CsvRow::RawFallback {
            source_row_id,
            span,
            reason: FallbackReason::UnclosedQuote,
        } => (*span, *source_row_id),
        row => panic!("unexpected row: {row:?}"),
    };
    assert_eq!(source_row_id, 2);
    assert_eq!(span.start_line, 3);
    assert_eq!(span.start, 22);
    assert_eq!(span.end, fs::metadata(&path).unwrap().len());

    let mut chunks = 0;
    let mut bytes = 0;
    visit_span_chunks(&path, span, 11, |chunk| {
        chunks += 1;
        bytes += chunk.len();
    })
    .unwrap();
    assert!(chunks > 1);
    assert_eq!(bytes as u64, span.len());
}

#[test]
fn oversized_record_is_file_backed_instead_of_owned() {
    let path = temp_csv("oversized");
    let mut file = File::create(&path).unwrap();
    file.write_all(b"value\n").unwrap();
    for _ in 0..(8 * 1024) {
        file.write_all(&[b'x'; 1024]).unwrap();
    }
    file.write_all(b"\n").unwrap();
    drop(file);

    let options = CsvScanOptions {
        max_record_bytes: 1024 * 1024,
        ..Default::default()
    };
    let mut row = None;
    scan_csv(&path, options, |value| {
        row = Some(value);
        ScanDecision::Continue
    })
    .unwrap();
    assert!(matches!(
        row,
        Some(CsvRow::RawFallback {
            reason: FallbackReason::RecordTooLarge { limit: 1_048_576 },
            ..
        })
    ));
    fs::remove_file(path).unwrap();
}

#[test]
fn discovers_a_decimal_after_nine_hundred_thousand_integer_rows() {
    let path = temp_csv("late-anomaly");
    let mut file = File::create(&path).unwrap();
    file.write_all(b"ClsPric\n").unwrap();
    for _ in 0..900_000 {
        file.write_all(b"1234\n").unwrap();
    }
    file.write_all(b"1234.56\n").unwrap();
    drop(file);

    let mut late_value = None;
    let report = scan_csv(&path, CsvScanOptions::default(), |row| {
        if let CsvRow::Mapped {
            source_row_id: 900_001,
            fields,
            ..
        } = row
        {
            late_value = fields[0].clone();
        }
        ScanDecision::Continue
    })
    .unwrap();

    assert_eq!(late_value.as_deref(), Some(b"1234.56".as_slice()));
    assert!(report.semantic_columns[0]
        .observed
        .contains(&ObservedKind::Integer));
    assert!(report.semantic_columns[0]
        .observed
        .contains(&ObservedKind::Decimal));
    fs::remove_file(path).unwrap();
}

#[test]
fn callback_can_stop_a_scan_cooperatively() {
    let path = fixture("flexible_rows.csv");
    let report = scan_csv(&path, CsvScanOptions::default(), |row| {
        if row.source_row_id() == 2 {
            ScanDecision::Stop
        } else {
            ScanDecision::Continue
        }
    })
    .unwrap();
    assert_eq!(report.rows_seen, 2);
    assert!(!report.completed);
}

/// Target for the Windows WorkingSet64 sampler documented in
/// `docs/ARCHITECTURE.md`. Fixture creation stays outside the measured process.
#[test]
#[ignore = "run explicitly with FASTXCEL_SPIKE_FILE for scale measurements"]
fn scans_external_fixture_for_memory_harness() {
    let path = PathBuf::from(
        std::env::var_os("FASTXCEL_SPIKE_FILE")
            .expect("FASTXCEL_SPIKE_FILE must name a generated CSV fixture"),
    );
    let report = scan_csv(&path, CsvScanOptions::default(), |_| ScanDecision::Continue).unwrap();
    assert!(report.completed);
    assert_eq!(report.structural_anomalies, 0);
    eprintln!("scanned {} data records", report.rows_seen);
}
