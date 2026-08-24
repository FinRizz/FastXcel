# FastXcel

**A blazing fast viewer for large datasets, optimized for financial time series.**
Built with Rust + [Polars](https://pola.rs) + [egui](https://github.com/emilk/egui).

[![CI](https://github.com/FinRizz/FastXcel/actions/workflows/ci.yml/badge.svg)](https://github.com/FinRizz/FastXcel/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey.svg)](#installation)

---

## Why FastXcel

Opening a multi-gigabyte OHLCV export in a spreadsheet is a bad time. Excel caps out at ~1M rows,
pandas wants the whole file in RAM, and both freeze the moment you scroll. FastXcel is a native
desktop viewer built for the specific job of *looking at* large tabular market data:

| Problem | FastXcel's approach |
| --- | --- |
| Row limits in spreadsheet tools | Paged reads — you choose the window, not the tool |
| Whole-file loads into memory | Polars `LazyFrame` + `.slice()` per page |
| Column names differ per vendor/exchange | Automatic canonicalization (`clspric` → `Close`) |
| UI stalls on large data | egui virtualized table — only visible rows are drawn |
| Slow ad-hoc filtering | Filters compile to Polars expressions, pushed into the scan |

## Features

- 📊 **CSV and Parquet** input, chosen through a native Windows file picker
- 🔍 **Live filtering** with a small comparison DSL (`Open > 100 & Volume > 10000`)
- 📝 **Column canonicalization** across common vendor and exchange schemas
- 📄 **Paged navigation** with a configurable page size (1,000 – 2,000,000 rows)
- 🖥️ **GPU-backed rendering** via `eframe`'s wgpu backend
- 🦀 **Single self-contained executable**, no runtime or interpreter to install

## Installation

### Prebuilt binary (Windows)

A prebuilt `fastxcel.exe` ships at the root of this repository — download it and double-click.
Nothing else to install.

### Build from source

Requires the [Rust toolchain](https://rustup.rs) (1.75 or newer).

```powershell
git clone https://github.com/FinRizz/FastXcel
cd FastXcel

cargo build --release
.\target\release\fastxcel.exe
```

> **Always build with `--release`.** A debug build of Polars is orders of magnitude slower and
> makes the app feel broken on any realistic dataset.

To build and launch in one step: `cargo run --release`.
For logging, set `RUST_LOG`: `$env:RUST_LOG="debug"; cargo run --release`.

Linux and macOS are not tested. The dependency stack (`eframe`, `rfd`, `polars`) is
cross-platform, so a source build will likely work with the usual GTK/Wayland development
packages installed — reports welcome.

## Usage

1. Click **📂 Open CSV/Parquet** and pick a file.
2. Use **⬅️ Prev** / **Next ➡️** to page through it.
3. Adjust **Page size** to trade memory and latency against how much you see at once.
4. Type a filter expression and press **🔍 Apply**.

The status line reports load time, column count, and the row range currently shown. A trailing
`+` means at least one more page exists.

### Filter expressions

The filter box takes a small, deliberately minimal DSL — *not* full Polars expression syntax.

| Element | Supported |
| --- | --- |
| Comparison | `>` `>=` `<` `<=` `==` `!=` |
| Logical | `&` (and), `\|` (or), left-associative, `&` binds tighter |
| Grouping | `( )` |
| Left operand | A column name (bare identifier: letters, digits, `_`) |
| Right operand | A number, or — for `==` / `!=` only — a bare word treated as a string |

```text
Open > 100 & Volume > 10000
(Close >= 250) | (Low < 100)
Symbol == TCS & Volume > 500000
```

**Filters reference canonical column names** (see below), because canonicalization runs before
the filter is applied.

Known limits of the DSL, all of which are open to contribution:

- No quoted strings — write `Symbol == TCS`, not `Symbol == "TCS"`
- No negative numbers, and no `NOT`
- No column-to-column comparison (`Open > Close` is rejected)
- **Invalid input fails silently.** A filter that does not parse is dropped and every row is
  shown, so a typo looks like "the filter did nothing" rather than an error.
- Trailing junk after a valid expression is ignored rather than rejected.

A filter that parses but is invalid for the data — comparing a text column to a number, for
instance — surfaces as a red `Load error:` line from Polars.

### Column canonicalization

Matching ignores case, spaces, dashes, and underscores, so `TRAD_DT`, `trad dt`, and `TradDt`
all resolve identically. Unrecognized columns pass through unchanged.

| Canonical name | Recognized aliases |
| --- | --- |
| `Open` | `open`, `opn`, `op`, `opnpric`, `opnprc` |
| `High` | `high`, `hgh`, `hghpric` |
| `Low` | `low`, `lw`, `lwpric` |
| `Close` | `close`, `cls`, `last`, `closeprice`, `clspric`, `sttlmpric` |
| `Volume` | `volume`, `vol`, `qty`, `totaltradedqty`, `ttltrfval` |
| `OpenInterest` | `opnintrst` |
| `ChangeInOpenInterest` | `chnginopnintrst` |
| `Symbol` | `tckrsymb` |
| `Expiry` | `fininstrmactlxprydt` |
| `TradeDate` | `traddt` |
| `Date` / `Timestamp` / `Time` | `date` / `timestamp` / `time` |

Adding a vendor schema means one line in `alias_map()` in `src/columns.rs`. That function is the
only place column naming is handled.

## Current status and known limitations

FastXcel is a working MVP at `0.1.0`. Being straight about where it stands:

- **Opening a file materializes it.** The schema is currently derived by collecting the whole
  frame, so peak memory on open scales with file size rather than page size. Paging *after* open
  is genuinely lazy. This is the single highest-impact fix outstanding — see
  [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
- **The page is re-collected every frame.** There is no result cache, so the Polars query runs
  continuously while the window is open. Lower the page size if the UI feels heavy.
- **No total row count.** Counting rows would mean scanning the file, so it is skipped by design;
  the `+` indicator is a heuristic (the page came back full).
- **Paging past the end** shows an empty table rather than clamping, and **Prev at offset 0** is a
  no-op.
- No sorting, no column pinning, no editing, no export. This is a viewer.
- CSV parsing assumes a header row, infers types from the first 512 rows, and ignores malformed
  rows silently.

Performance numbers are deliberately absent from this README until they are reproducible on a
published benchmark. If you measure something, a PR adding the harness is very welcome.

## Architecture

Four modules and one data path:

```
src/
├─ main.rs            # eframe entry point, logger init
├─ app.rs             # UltraFastApp — all UI state, top bar, frame loop
├─ data.rs            # DataEngine — LazyFrame ownership, open_path, fetch_page
├─ columns.rs         # alias_map/canonicalize_columns + the filter DSL parser
└─ ui/table.rs        # DataTableWidget — virtualized egui table, dtype formatting
```

Every query goes through `DataEngine::fetch_page`, which canonicalizes columns, applies the
parsed filter, slices the page, and collects. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
for the full walkthrough, including the parser grammar and the performance traps to avoid.

## Roadmap

- [ ] Cache collected pages; invalidate on offset / page size / filter change
- [ ] Derive schema without materializing the frame (`collect_schema`)
- [ ] Quoted string literals, negative numbers, and `NOT` in the filter DSL
- [ ] Surface parse errors instead of silently dropping the filter
- [ ] Column pinning and click-to-sort
- [ ] Schema and column-statistics panel
- [ ] Candlestick chart view for OHLCV data
- [ ] Additional formats (Arrow IPC, JSONL)
- [ ] Benchmark harness with published numbers

## Contributing

Contributions are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md) for setup, build
commands, and the conventions this repo follows; every item in the roadmap above is fair game,
and the "known limitations" list is effectively the good-first-issue queue.

By participating you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

To report a security issue, follow [SECURITY.md](SECURITY.md) — please do not open a public issue.

Release history lives in [CHANGELOG.md](CHANGELOG.md).

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgements

Built on [Polars](https://github.com/pola-rs/polars), [egui / eframe](https://github.com/emilk/egui),
and [rfd](https://github.com/PolyMeilex/rfd).
