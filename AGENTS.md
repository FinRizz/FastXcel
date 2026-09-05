# Repository Guidelines

## Project Structure

FastXcel is a Windows-first Rust desktop viewer for large CSV and Parquet datasets. The crate lives at the repository root. `src/main.rs` initializes logging and launches the egui application; `src/app.rs` owns UI state and paging; `src/data.rs` handles Polars lazy scans and page collection; `src/columns.rs` contains column canonicalization and the filter parser; and `src/ui/table.rs` renders the virtualized table. Treat `src/mod.rs` as dead code: `main.rs` is the crate root. Design notes and implementation plans belong in `docs/`. The checked-in `fastxcel.exe` is the release artifact.

## Build, Run, and Verification

Use Rust 1.75 or newer, with the MSVC toolchain on Windows.

```powershell
cargo check                  # fast type-check while iterating
cargo build --release        # optimized binary in target\release
cargo run --release          # build and launch the GUI
cargo clippy --all-targets   # lint all targets
cargo test                   # run the test suite when tests exist
```

Always launch with `--release`; debug Polars builds are impractically slow on realistic data. Enable diagnostics with `$env:RUST_LOG="debug"; cargo run --release`. `Cargo.lock` is intentionally ignored, so investigate dependency drift when builds differ across machines.

## Coding Conventions

Follow Rust 2021 conventions: four-space indentation, `snake_case` functions/modules, `PascalCase` types, and `SCREAMING_SNAKE_CASE` constants. Keep responsibilities in their existing modules. Add vendor column aliases only in `alias_map()` and new displayed data types in `cell_str()`. The repository is not globally rustfmt-clean; format only files or selections you touch, and isolate formatting-only changes.

## Testing and Manual Checks

There is currently no committed test suite. Add unit tests beside pure logic, especially parser tests in `src/columns.rs`, using descriptive names such as `parse_filter_respects_and_precedence`. Before submitting, run `cargo check` and `cargo clippy --all-targets`. Manually open both CSV and Parquet samples, page in both directions, change page size, and test matching, nonmatching, and invalid filters. Do not commit large fixture datasets.

## Commits and Pull Requests

History is minimal and does not establish a strong convention. Prefer imperative subjects under roughly 72 characters, for example `Clamp page offset at EOF`; Conventional Commit prefixes are optional. Use focused branches such as `feat/column-pinning` or `fix/filter-errors`. Keep each PR to one behavioral change, describe verification steps, link relevant issues, and include screenshots for visible UI changes. Update `README.md` for user-facing behavior and add an entry under `CHANGELOG.md`'s `Unreleased` section.

## Security

Treat opened files as untrusted input. Avoid panics or unbounded allocation on malformed data. Report vulnerabilities privately according to `SECURITY.md`, never in a public issue.
