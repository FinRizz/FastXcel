# Architecture

FastXcel is a Rust 2021 library plus a thin Windows desktop launcher. `src/lib.rs` exports the
application, data, column, and UI modules; `src/main.rs` initializes logging and starts eframe.

## Current query path

`UltraFastApp::update` runs once per rendered frame. When a file is open it calls
`DataEngine::fetch_page`, which clones the stored Polars `LazyFrame`, canonicalizes column names,
parses and applies the filter, slices at the current offset, and calls `collect`. The table then
virtualizes drawing of the collected page. There is currently no page cache, so an unchanged
frame repeats that query. `open_path` also currently calls `collect` to derive the schema, which
materializes the dataset during open. These are known constraints, not intended architecture.

```text
eframe update
  -> DataEngine::fetch_page
     -> canonicalize columns -> parse/apply filter -> slice -> collect
  -> DataTableWidget::show
```

CSV and Parquet both enter through `DataEngine::open_path`. CSV assumes a header, infers from the
first 512 rows, and currently asks Polars to ignore malformed records. Parquet remains typed.

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
