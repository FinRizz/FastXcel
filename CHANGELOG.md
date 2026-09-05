# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0]

### Fixed

- Preserve temporal and other supported Parquet columns using full Polars type support;
  remove the fallback that hid timestamps and discarded filters.
- Display nanosecond timestamps with fixed offsets and IANA timezone conversion.
- Avoid internal row-ID column collisions, detect the final page accurately, reset paging
  when filters change, clear stale results on query failure, and resolve selected cells
  by their source row identity.

- Recover pages from Parquet files whose unsupported timestamp columns prevent Polars from
  materializing a series, including timezone-qualified nanosecond timestamps.
- Restore horizontal scrolling for wide data tables while retaining resizable column widths.

### Added

- A library crate seam and parser characterization tests.
- Project documentation: `LICENSE`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`,
  `CHANGELOG.md`, `docs/ARCHITECTURE.md`, GitHub issue/PR templates, and a CI workflow.
- Expanded `README.md` with a filter-expression reference, the column alias table, and an
  explicit list of current limitations.
- Wide tables now keep their content-sized column widths so horizontal scrolling stays available
  instead of squeezing every column into the viewport.
- Windows release builds now embed the FastXcel icon into the `.exe` and the release workflow
  runs a smoke test after packaging.
- Windows builds use the provided `assets/fastxcel.ico` for the executable and app window chrome.

## 0.1.0 repository milestone - 2025-09-02

Initial code milestone. This version was never published as a reproducible tagged release; the
first CI-built release is planned for `v0.2.0`.

### Added

- Native desktop viewer for CSV and Parquet files, built on Polars `LazyFrame` and egui/eframe
  with the wgpu backend.
- Paged navigation with a configurable page size (1,000 – 2,000,000 rows) that slices lazily
  rather than loading whole files.
- Virtualized table widget rendering only visible rows, with per-dtype cell formatting and
  scientific notation for large floats.
- Column canonicalization mapping vendor and exchange OHLCV names (`opnpric`, `clspric`,
  `tckrsymb`, `ttltrfval`, and others) onto canonical names.
- Filter DSL supporting `>`, `>=`, `<`, `<=`, `==`, `!=`, `&`, `|`, and parentheses, compiled to
  Polars expressions and pushed into the scan.
- Native Windows file picker via `rfd`, and `RUST_LOG`-controlled logging via `env_logger`.
- Release profile tuned for size and speed: `opt-level = 3`, LTO, one codegen unit,
  `panic = "abort"`, symbols stripped.

[Unreleased]: https://github.com/FinRizz/FastXcel/commits/HEAD
