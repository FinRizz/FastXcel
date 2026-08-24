# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

FastXcel — a native desktop viewer for large CSV/Parquet datasets, aimed at financial OHLCV time series. Rust + Polars (data engine) + egui/eframe with the wgpu backend (GUI). Windows-first; the only committed build artifact is `fastxcel.exe` at the repo root.

## Commands

```powershell
cargo build --release        # release build → target\release\fastxcel.exe
cargo run --release          # build + launch the GUI
cargo check                  # fast type-check without linking (use while iterating)
cargo clippy --all-targets
cargo fmt
```

Always use `--release` when running. Debug-profile Polars is orders of magnitude slower and makes the app feel broken on any real dataset.

`RUST_LOG=debug cargo run --release` enables logging — `main.rs` calls `env_logger::init()`, so `RUST_LOG` is the only logging control.

There are currently **no tests** in the repo. Once one exists, run a single test with `cargo test --lib <test_name>`. Note the release profile sets `panic = 'abort'`, so `#[should_panic]` tests must run under the default dev profile.

`Cargo.lock` is gitignored, so dependency versions can drift between checkouts — pin in `Cargo.toml` if a build breaks.

## Architecture

Four modules, one data path. `main.rs` declares them (`mod app; mod data; mod columns; mod ui;`) and hands `UltraFastApp` to `eframe::run_native`.

**`data.rs` — `DataEngine`** owns an `Option<LazyFrame>`. `open_path` dispatches on file extension to `LazyCsvReader` (header assumed, `ignore_errors`, 512-row schema inference) or `LazyFrame::scan_parquet`; anything else is a `ComputeError`. `fetch_page(offset, limit, filter)` is the single query entry point: canonicalize columns → optionally apply the parsed filter → `.slice(offset, limit)` → `.collect()`. Total row count is deliberately never computed; the "more pages" flag is the heuristic `df.height() == limit`.

**`columns.rs`** does two unrelated jobs. `canonicalize_columns` renames vendor-specific OHLCV column names (`opnpric`, `clspric`, `tckrsymb`, `ttltrfval`, …) to canonical ones (`Open`, `Close`, `Symbol`, `Volume`) by lowercasing and stripping spaces/dashes/underscores, then emitting a `select` projection of aliased `col()` exprs. Add new vendor schemas to `alias_map()` — it is the single place column naming is handled. The rest of the file is a hand-written recursive-descent parser (`ExprBuilder::parse` → `tokenize` → `parse_or`/`parse_and`/`parse_atom`/`parse_cmp`) producing a Polars `Expr`.

**`app.rs` — `UltraFastApp`** holds all UI state (`page_size` default 100_000, `current_offset`, `filter_query`, `open_path`). Its `update` draws the top bar, then calls `engine.fetch_page(...)` and passes the resulting `DataFrame` to the table widget.

**`ui/table.rs` — `DataTableWidget`** wraps `egui_extras::TableBuilder` with a leading row-index column plus one `Column::remainder()` per data column. `cell_str` matches on `DataType` to stringify; `format_float` switches to scientific notation at ≥1e6. Extending dtype support means adding arms to `cell_str`.

## Behaviors worth knowing before changing things

- **`fetch_page` runs on every frame.** `app.rs` calls it unconditionally inside `update`, and `update` ends with `ctx.request_repaint()`, so a full Polars `collect()` of the page happens continuously. Caching the collected page (invalidating on offset/page-size/filter change) is the obvious win and the main reason the "Apply" filter button has an empty click handler — it only needs to trigger a repaint that would happen anyway.
- **Loading is not actually lazy yet.** `open_path` grabs the schema via `lf.clone().collect()?.schema()`, which materializes the whole file, and `canonicalize_columns` re-derives the schema via `lf.clone().fetch(10)` on every page fetch. Use `LazyFrame::schema()`/`collect_schema()` if you touch this — the README's lazy-loading and load-time claims depend on it.
- **The filter DSL is not Polars expression syntax**, despite the UI hint. It supports only `Ident OP Number` and `Ident ==|!= Ident`, combined with `&`, `|`, and parentheses. No quoted strings, no negative numbers (the tokenizer only consumes digits and `.`), no `NOT`, no column-to-column comparison. Parse failure is silent: `ExprBuilder::parse` returns `None` and the filter is dropped, so bad input looks like "filter did nothing". Filters reference **canonical** names, since canonicalization runs first.
- **`src/mod.rs` is dead.** With `main.rs` as the crate root, `src/mod.rs` is never compiled. Ignore it, or delete it — do not add module declarations there expecting them to take effect.
- **Prev/Next has a bug worth not replicating**: `self.current_offset >= self.page_size` is checked after the click, so clicking Prev at offset 0 silently no-ops. Next saturates upward with no upper bound, so paging past EOF shows an empty table.
- **egui 0.29 API**: `DragValue::range` (not `clamp_range`), and the `run_native` creation closure returns `Ok(Box::new(app))`. Version-specific breakage here is common when copying snippets from other egui releases.
