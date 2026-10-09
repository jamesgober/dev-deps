# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.2] - 2026-10-09

Tool-invocation and version-comparison fixes from a review pass, plus
the MSRV rollback to Rust 1.75. The rollback follows `dev-fixtures`
0.9.5 swapping `tempfile` for `mod-tempdir` 1.0, which removed the
`getrandom 0.4.2 -> edition2024` chain that held the dev-* collection
at 1.85.

### Added

- `VERSION` constant with the crate version as compiled, so tools that
  bundle this crate can report what is actually linked.

### Fixed

- `cargo outdated` without `--workspace` listed every transitive
  dependency as a `parent->child` row, mostly with a latest version of
  `Removed`, and each became a junk finding. `--root-deps-only` is now
  always passed, and rows whose version fields are not versions
  (`Removed`, `---`) or that name a transitive path are skipped.
- `major_behind` ignored Cargo's rule for 0.x versions, so
  `rand 0.7.3 -> 0.10.3` counted as 0 majors behind (`Info`) and could
  never escalate. The left-most non-zero component is now the breaking
  one, so that example is 3 behind. Build metadata such as `+wasi` is
  parsed.
- `cargo udeps` ran without `--all-targets`, so unused
  dev-dependencies were never found and a dependency used only by tests
  was flagged as unused. The flag is now passed.
- The parse-error preview sliced at byte 200 and could panic inside a
  multi-byte character.

### Changed

- When workspace members pin different versions of one crate, the
  outdated finding kept is the one furthest behind (it was arbitrary).
- Error messages name the tool that failed and its exit status.
- The unused `tempfile` dev-dependency is gone; it kept
  `cargo +1.75 test` from resolving.
- `rust-version` lowered from `1.85` to `1.75`. CI's MSRV job now
  builds on 1.75 against an MSRV-compatible lockfile; it was still
  pinned to 1.85.

### Documentation

- README Scopes table lists the real tool flags, explains how "majors
  behind" is counted, and corrects the `allow` comment; MSRV section
  says 1.75.
- `docs/API.md`: stale `kind: String` and missing fields fixed, builder
  methods added. `OutdatedDep::current` says the version comes from
  `Cargo.lock`.

[0.9.2]: https://github.com/jamesgober/dev-deps/releases/tag/v0.9.2

## [0.9.1] - 2026-05-12

Documentation and SEO pass. No code changes.

### Changed

- README header standardized: Rust logo image, MSRV badge between CI and docs.rs (was at the end of the badge list, lowercase label), copyright block at bottom.
- Subtitle now reads `DEPENDENCY HYGIENE FOR RUST CRATES` (was `DEPENDENCY HEALTH FOR RUST`). `Hygiene` is closer to how developers describe this work; `crates` narrows the audience.
- Tagline rewritten to lead with what the crate detects (unused, outdated, major-version-lag) and the wrapped tools.
- `## The dev-* suite` retitled to `The dev-* collection` and expanded with the full 14-crate map.
- `Cargo.toml` description rewritten: explicit about what's detected and what tools are wrapped.
- `Cargo.toml` keywords retuned: dropped `verification` and `ai-tools`, added `udeps` and `ci` for crates.io search.

### Added

- "Part of the `dev-*` verification collection" block on the README, under the intro, linking the umbrella `dev-tools` crate.

[0.9.1]: https://github.com/jamesgober/dev-deps/releases/tag/v0.9.1

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
