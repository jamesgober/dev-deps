# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-05-11

### Added

- Initial crate skeleton.
- `DepScope` enum: `Unused`, `Outdated`, `All`.
- `DepCheck` builder with `new`, `scope`, `dep_scope`, `execute`.
- `UnusedDep` and `OutdatedDep` finding types.
- `DepResult` with `unused`, `outdated`, `total_findings`, `into_report`.
- `DepError` for tool-missing / subprocess / parse failures.
- Smoke tests covering scope selection, empty results, mixed findings.

### Note

This is the name-claim release. The actual `cargo-udeps` and
`cargo-outdated` subprocess integrations land in `0.9.1`.

[Unreleased]: https://github.com/jamesgober/dev-deps/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/jamesgober/dev-deps/releases/tag/v0.9.0
