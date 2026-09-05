# Contributing to FastXcel

Thanks for your interest. FastXcel is small and early, which means most contributions land
quickly and touch code that is easy to hold in your head — the whole crate is around 450 lines.

By participating you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting set up

You need the [Rust toolchain](https://rustup.rs) 1.88 or newer. On Windows, the MSVC toolchain
plus the Visual Studio Build Tools ("Desktop development with C++") is the tested configuration.

```powershell
git clone https://github.com/FinRizz/FastXcel
cd FastXcel

cargo check                  # fast type-check, use this while iterating
cargo build --release        # full build → target\release\fastxcel.exe
cargo run --release          # build and launch
```

The first build compiles Polars and takes a while — several minutes is normal. After that,
`cargo check` is your friend.

Two things that will save you time:

- **Always run with `--release`.** A debug build of Polars is slow enough that the app looks
  broken on any real dataset. If you are debugging behavior rather than speed, add
  `[profile.dev] opt-level = 1` locally rather than reaching for a bare debug build.
- **Keep `Cargo.lock` unchanged unless dependencies intentionally change.** It is tracked so local
  builds and CI resolve the same versions.

### Logging

`main.rs` initializes `env_logger`, so `RUST_LOG` is the only control:

```powershell
$env:RUST_LOG="debug"; cargo run --release
```

## Verifying a change

Parser characterization tests live beside the implementation in `src/columns.rs`. Add focused
unit tests there for grammar changes and integration tests under `tests/` for public library
behavior:

```powershell
cargo test                       # whole suite
cargo test --lib parse_filter    # a single test by name
```

Run the automated checks first:

```powershell
cargo check
cargo clippy --all-targets
cargo run --release
```

Then open a real CSV *and* a real Parquet file, page forward and back, change the page size, and
apply a filter that matches and one that does not. Please say in your PR which files and which
filters you exercised.

### About `cargo fmt`

The repo is **not** currently rustfmt-clean. The parser helpers in `src/columns.rs`
(`parse_or`, `parse_and`, `parse_atom`) are hand-compacted onto single lines, and a blanket
`cargo fmt` expands them into a large unrelated diff.

Format only the code you touched — most editors can format a selection or the current file — and
keep formatting-only changes in their own commit so reviewers can separate them from behavior.
CI runs a formatting check as advisory only; it will not block your PR.

## Where things live

| File | Responsibility |
| --- | --- |
| `src/lib.rs` | Library module root exposed to integration tests and the launcher |
| `src/main.rs` | Thin `eframe::run_native` launcher and logger initialization |
| `src/app.rs` | `UltraFastApp` — all UI state, top bar, per-frame `update` |
| `src/data.rs` | `DataEngine` — owns the `LazyFrame`; `open_path`, `fetch_page` |
| `src/columns.rs` | `alias_map` / `canonicalize_columns`, plus the filter DSL parser |
| `src/ui/table.rs` | `DataTableWidget` — virtualized table, per-dtype cell formatting |

Common changes map to exactly one place:

- **Support a new vendor's column names** → add entries to `alias_map()` in `src/columns.rs`.
  Keys are lowercased with spaces, dashes, and underscores stripped.
- **Render a new Arrow dtype** → add an arm to `cell_str` in `src/ui/table.rs`.
- **Extend the filter syntax** → `tokenize` and `parse_cmp` in `src/columns.rs`.
- **Support a new file format** → the extension match in `DataEngine::open_path`.

`src/mod.rs` is legacy dead code; the active module root is `src/lib.rs`. Do not extend
`src/mod.rs`.

Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) before any non-trivial change — it documents
the per-frame query behavior and the parser grammar, both of which are easy to break by accident.

## Good first issues

Drawn from the known limitations in the README, roughly easiest first:

1. **Clamp `Next` at the end of the file** so paging past EOF stops instead of showing an empty
   table, and **fix `Prev` at offset 0** (the guard in `app.rs` is evaluated after the click, so
   the button silently does nothing).
2. **Add the missing price aliases** — `closeprice` is recognized but `openprice`, `highprice`,
   and `lowprice` are not.
3. **Report filter parse errors in the UI.** `ExprBuilder::parse` returns `None` on bad input and
   the filter is silently dropped, which reads as "the filter did nothing".
4. **Reject trailing tokens.** `parse_expr` ignores anything after a valid expression, so
   `Open > 100 zzz` silently parses as `Open > 100`.
5. **Unit-test the parser** — precedence, associativity, and the rejection cases.
6. **Quoted string literals, negative numbers, and `NOT`** in the DSL.
7. **Cache the collected page**, keyed on offset, page size, and filter. Today `fetch_page` runs
   a full Polars `collect()` on every frame.
8. **Derive the schema without materializing the frame.** `open_path` calls `collect()` on the
   whole file purely to read its schema, which is why peak memory on open scales with file size.

Items 7 and 8 are the highest-impact changes in the codebase; they are last only because they
need the most care.

## Pull requests

1. Fork, then branch from `main` — `feat/column-pinning`, `fix/prev-button-guard`.
2. Keep the PR focused. One behavioral change per PR; unrelated refactors and reformatting go in
   separate commits or separate PRs.
3. Write commit subjects in the imperative mood, under ~72 characters: `Clamp page offset at EOF`.
   [Conventional Commits](https://www.conventionalcommits.org) prefixes are welcome, not required.
4. Update `README.md` if you change user-facing behavior, and add a `CHANGELOG.md` entry under
   `## [Unreleased]`.
5. Fill in the PR template — especially *how you verified the change*, since CI cannot click
   buttons.

Small PRs get reviewed faster. If you are planning something large, open an issue first so we can
agree on the approach before you write it.

## Reporting bugs and requesting features

Use the [issue templates](https://github.com/FinRizz/FastXcel/issues/new/choose). For bugs, the
most useful thing you can include is the shape of the input: format, approximate row and column
count, the dtypes involved, and the exact filter expression. A minimal file that reproduces the
problem is even better.

Security issues follow a different path — see [SECURITY.md](SECURITY.md).
