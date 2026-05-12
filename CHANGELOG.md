# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-05-12

Foundation release. Replaces the `0.1.0` name-claim with full
`cargo-udeps` + `cargo-outdated` integration.

### Added

- Real `cargo +nightly udeps --output json` subprocess integration. Parses the `unused_deps` map and emits one `UnusedDep` per kind (normal / development / build) per package. Tool absence (or missing nightly toolchain) surfaces as `DepError::UdepsToolNotInstalled`.
- Real `cargo outdated --format json` subprocess integration with streaming JSON parsing — handles the concatenated multi-workspace-member shape (one JSON object per member, no separators). Tool absence surfaces as `DepError::OutdatedToolNotInstalled`.
- Severity policy per REPS § 5:
  - Unused dependency → `Warning`.
  - Outdated, 0–1 major behind → `Info`.
  - Outdated, 2+ majors behind → `Warning`.
  - `DepCheck::escalate_at_majors(n)` lifts the verdict to `Error` when at least `n` majors behind, turning the produced `CheckResult` into a failing one.
- `DepKind` enum (`Normal`, `Development`, `Build`) replaces the loose `String` field on `UnusedDep` / `OutdatedDep`. `as_str` returns the matching `Cargo.toml` section name.
- `DepCheck` builder methods: `in_dir(path)`, `workspace()`, `exclude(pattern)`, `allow(name)`, `allow_all(iter)`, `severity_threshold(sev)`, `escalate_at_majors(n)`, `subject()`, `subject_version()`.
- Deterministic post-processing: allow-list and exclude filters, severity-threshold filter, sort by `crate_name` (then `kind` for `UnusedDep`), dedup by `(crate_name, kind)` / `crate_name`.
- `UnusedDep::severity()` and `OutdatedDep::severity(escalate_at)` surface the per-finding severity that `into_report` uses.
- `DepResult` methods: `total_findings`, `unused_count`, `outdated_count`, `worst_severity`.
- `DepResult::into_report` now emits one `CheckResult` per finding named `deps::unused::<crate>` or `deps::outdated::<crate>`. Each is tagged `deps` plus a kind tag (`unused` / `outdated`). Outdated findings carry `Evidence::numeric_int("major_behind", N)` and `Evidence::KeyValue("finding", {crate, current, latest, kind?})`. The escalated `Error`-severity findings become `CheckResult::fail`; everything else remains `CheckResult::warn`.
- New `udeps` and `outdated` modules holding the subprocess invocation + JSON parsers, plus a `producer` module with `DepProducer` — a `dev_report::Producer` adapter that maps subprocess failures to a single `Severity::Critical` `CheckResult`.
- Examples: `basic.rs` (full check, graceful tool-missing handling), `unused_only.rs`, `outdated_only.rs`, `producer.rs` (gated by `DEV_DEPS_EXAMPLE_RUN`).
- 29 unit tests across `lib.rs`, `udeps.rs`, `outdated.rs`, `producer.rs`. Coverage includes: severity policy per finding kind, escalation threshold behavior, JSON parsing for both tools (including the concatenated multi-workspace shape), `DepKind` round-trips, garbage-input rejection, allow-list and exclude filtering, dedup ordering, and `worst_severity` picking across categories.
- 11 integration tests in `tests/smoke.rs` plus one `#[ignore]`d real-subprocess test (documents the `CARGO_TARGET_DIR` workaround).

### Changed

- `cargo install cargo-udeps cargo-outdated` and the nightly toolchain (for `cargo-udeps`) are now real runtime requirements (previously declared but the code didn't actually invoke them).
- README rewritten: removes the "subprocess integration lands in 0.9.1" disclaimer, documents the allow-list / threshold / escalation workflow, describes the JSON shape of `UnusedDep` / `OutdatedDep`, and pins MSRV at 1.85.
- REPS.md tightened: the "SHOULD provide" items (subprocess integration for both tools, major-version-lag threshold gating, workspace-aware checks) are now MUST-have for 0.9.x.
- CI workflow: new `integration` job installs `cargo-udeps`, `cargo-outdated`, and the nightly toolchain via `taiki-e/install-action` + `dtolnay/rust-toolchain` and verifies both tools run. `actions/checkout@v5` everywhere; path-dep `../dev-report` cloned in every job.

### Dependencies

- Added: `serde` 1.0 (derive feature), `serde_json` 1.0. Required for parsing both tools' JSON output and for serializing `DepResult` / `UnusedDep` / `OutdatedDep`.
- Added: `tempfile` 3 as a `dev-dependency`.

### Note

`0.1.0` was a name-claim publish with a stub `execute()` returning empty findings. The public API surface changed in two breaking ways:

1. `UnusedDep::kind` is now `DepKind` instead of `String`.
2. `OutdatedDep` gained `kind: Option<DepKind>`.

The constructor surface (`DepCheck::new`, `DepScope` variants, `DepResult::into_report`) is otherwise unchanged. See the migration block in the README.

[Unreleased]: https://github.com/jamesgober/dev-deps/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/jamesgober/dev-deps/releases/tag/v0.9.0
[0.1.0]: https://github.com/jamesgober/dev-deps/releases/tag/v0.1.0
