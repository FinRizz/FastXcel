# Architecture

FastXcel is a Rust 2021 library plus a thin Windows desktop launcher. `src/lib.rs` exports the
application, data, column, and UI modules; `src/main.rs` initializes logging and starts eframe.

## Current query path

`UltraFastApp::update` runs once per rendered frame. When a file is open it calls
`DataEngine::fetch_page`, which clones the stored Polars `LazyFrame`, canonicalizes column names,
parses and applies the filter, slices at the current offset, and calls `collect`. The table then
virtualizes drawing of the collected page. There is currently no page cache, so an unchanged
frame repeats that query. `open_path` now collects schema metadata without materializing the
dataset, but the page fetch path still collects the active slice. These are known constraints, not
intended architecture.

```text
eframe update
  -> DataEngine::fetch_page
     -> canonicalize columns -> parse/apply filter -> slice -> collect
  -> DataTableWidget::show
```

CSV and Parquet both enter through `DataEngine::open_path`. CSV assumes a header, infers from the
first 512 rows, and the ingestion spike now uses a synchronous byte scanner plus a small
bounded parser so malformed records can stay inspectable. Parquet remains typed.

## Filter grammar

The parser in `src/columns.rs` implements this grammar:

```text
expression  := or_term
or_term     := and_term ("|" and_term)*
and_term    := atom ("&" atom)*
atom        := comparison | "(" or_term ")"
comparison  := identifier operator (number | identifier)
operator    := ">" | ">=" | "<" | "<=" | "==" | "!="
```

`&` binds more tightly than `|`; repeated logical operators associate left. String identifiers
are valid only with `==` and `!=`. The current parser intentionally returns `None` on malformed
input and accepts trailing tokens after a valid expression. Characterization tests preserve
these behaviors until the result-returning parser work changes them deliberately.

## CSV ingestion spike findings

The preservation boundary uses a synchronous byte boundary scanner followed by a small bounded
record parser. The scanner preserves exact half-open file byte spans, detects an unterminated quoted
tail, and enforces a hard cap on owned record bytes. The record parser preserves invalid UTF-8,
quoted newlines, quoted commas, and escaped quotes as byte content while still rejecting malformed
record structure. This validates the format split in KTD2; Parquet should continue through Polars
rather than acquiring CSV recovery concepts.

Data-record identities are assigned sequentially after the header and begin at one. A row with
missing fields is padded with explicit missing values. Extra fields, invalid UTF-8, parser errors,
and records over the 1 MiB ownership limit become `RawFallback` values carrying a `ByteSpan` and
reason. An open quote at EOF produces one fallback spanning from the damaged record's first
physical line to EOF; later physical lines are not guessed to be records. `visit_span_chunks`
reads such spans with caller-bounded memory. The scanner retains only its 64 KiB input buffer,
semantic observation sets, and at most one 1 MiB record, so its allocation shape is independent
of file length. The callback can stop cooperatively between records.

The focused tests cover empty/header-only files, flexible rows, quoted commas and escaped quotes,
empty fields versus empty records, embedded newlines, invalid encoding, unclosed quotes, an 8 MiB
fallback, stable byte/row positions, a decimal at data row 900,001, and cooperative stop.
`scans_external_fixture_for_memory_harness` is an ignored release-test target for the scale
measurement. On Windows, generate fixed-schema
64 MiB, 256 MiB, and 1 GiB files outside the measured process; build the release test once; run one
warm-up plus three measured processes per size with `FASTXCEL_SPIKE_FILE` set; and sample the child
process's `WorkingSet64` every 100 ms. Subtract the pre-scan idle working set and compare median
peaks. The gate passes when growth from 64 MiB to 1 GiB is at most 64 MiB.

The spike is intentionally synchronous and is not the production reader. It does not implement
background progress, paging, filter evaluation, or semantic-authority transitions. It recognizes
LF and CRLF record terminators; support for legacy CR-only files must be decided before promoting
the scanner. Memory measurements were run on an MSVC host with `link.exe`; the release harness
linked successfully, and U2's empirical memory gate is recorded as passed.
