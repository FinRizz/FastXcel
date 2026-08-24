# Security Policy

## Supported versions

FastXcel is pre-1.0. Only the latest release on `main` receives security fixes.

| Version | Supported |
| --- | --- |
| 0.1.x | ✅ |
| < 0.1 | ❌ |

## Reporting a vulnerability

**Please do not open a public issue for a security problem.**

Report privately through either channel:

1. [GitHub private vulnerability reporting](https://github.com/FinRizz/FastXcel/security/advisories/new)
   — preferred, keeps the discussion attached to the repository.
2. Email **ayush.tripathi.8757@gmail.com** with `FastXcel security` in the subject.

Please include the FastXcel version and commit, your OS and Rust version, a description of the
issue and its impact, and — where possible — a minimal input file or steps that reproduce it.

You can expect an acknowledgement within 7 days and a status update at least every 14 days while
the report is open. Once a fix ships, you will be credited in the advisory and `CHANGELOG.md`
unless you prefer otherwise. Please give us a reasonable window to release a fix before
disclosing publicly.

## Threat model

FastXcel is a local desktop viewer with no network I/O, no telemetry, and no auto-update. The
realistic attack surface is **the contents of a file a user opens**, so the following are in
scope:

- Memory-safety issues, panics, or unbounded allocation triggered by a malformed or hostile
  CSV/Parquet file (parsing is delegated to Polars and Arrow, so such reports may be forwarded
  upstream).
- Path handling around the file picker.
- Anything that turns opened data into executed code.

Out of scope:

- Resource exhaustion from opening a legitimately huge file. Peak memory currently scales with
  file size on open — this is a known performance limitation documented in the README, not a
  vulnerability.
- Filter expressions producing Polars errors. The DSL only builds comparison expressions; it
  cannot execute arbitrary code.
- Vulnerabilities in dependencies with no reachable path from FastXcel. Please report those
  upstream, though a heads-up here is appreciated.
