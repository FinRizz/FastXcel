# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Project documentation: `LICENSE`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`,
  `CHANGELOG.md`, `docs/ARCHITECTURE.md`, GitHub issue/PR templates, and a CI workflow.
- Expanded `README.md` with a filter-expression reference, the column alias table, and an
  explicit list of current limitations.

## [0.1.0] - 2025-09-02

Initial release.

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

[Unreleased]: https://github.com/FinRizz/FastXcel/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/FinRizz/FastXcel/releases/tag/v0.1.0
