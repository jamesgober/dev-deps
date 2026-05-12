# dev-deps — Project Specification (REPS)

> Rust Engineering Project Specification.
> Normative language follows RFC 2119.

## 1. Purpose

`dev-deps` MUST check dependency health (unused and outdated) and emit
findings as `dev-report::Report`. Output MUST be machine-readable so
AI agents and CI gates can act on results without parsing free-form
logs.

## 2. Scope

This crate MUST provide:

- A `DepScope` enum (`Unused`, `Outdated`, `All`).
- A `DepCheck` builder with `scope`, `in_dir`, `workspace`,
  `exclude`, `allow`, `allow_all`, `severity_threshold`,
  `escalate_at_majors`, `subject`, `subject_version`, and `execute`
  methods.
- A `DepKind` enum (`Normal`, `Development`, `Build`) for declaring
  the section of `Cargo.toml` a dependency lives under.
- `UnusedDep` and `OutdatedDep` finding types with severity helpers.
- `cargo-udeps` invocation + JSON parsing.
- `cargo-outdated` invocation + JSON parsing (including the concatenated
  multi-workspace-member shape).
- A `DepResult` with `total_findings`, `unused_count`,
  `outdated_count`, `worst_severity`, and `into_report`.
- Major-version-lag escalation through `DepCheck::escalate_at_majors`.
- Workspace-aware checks via `DepCheck::workspace`.
- A `DepProducer` adapter mapping subprocess failures to a single
  failing `CheckResult` rather than panicking.

This crate MAY provide later:

- Per-workspace-member breakdown in the produced `Report`.
- Yanked-version detection.
- HTTP-served advisory feeds for crate ownership tracking.

This crate MUST NOT:

- Replace `cargo-udeps` or `cargo-outdated`. We wrap them.
- Edit `Cargo.toml`. Reporting is the contract; remediation is the
  user's choice.
- Network-fetch package metadata directly. The wrapped tools handle
  network I/O.

## 3. Determinism

Same project + same lockfile MUST produce the same findings list, in
the same order. Findings MUST be sorted ascending by `crate_name`,
breaking ties for `UnusedDep` by `kind`. Findings with identical
`(crate_name, kind)` MUST be deduplicated. Two diffs of the same
input MUST be byte-equal.

## 4. Tool dependencies

`cargo-udeps` MUST be installed externally. It also requires the
nightly toolchain (`rustup toolchain install nightly`). The crate
invokes it via `cargo +nightly udeps --output json`. Detection of a
missing tool (or missing nightly) produces
`DepError::UdepsToolNotInstalled`.

`cargo-outdated` MUST be installed externally. The crate invokes it
via `cargo outdated --format json`. Detection of a missing tool
produces `DepError::OutdatedToolNotInstalled`.

Subprocess failures (non-zero exit *and* empty stdout) MUST surface
as `DepError::SubprocessFailed(stderr)`. Parse failures MUST surface
as `DepError::ParseError(detail)`. Neither MUST cause a panic.

Both tools return non-zero exit codes when they find issues — this is
the success path for `dev-deps`. The crate MUST parse stdout
regardless of exit code as long as stdout is non-empty.

## 5. Severity policy

| Finding type                                  | Severity              |
|-----------------------------------------------|-----------------------|
| Unused dependency                             | `Warning`             |
| Outdated, 0–1 major behind                    | `Info`                |
| Outdated, 2+ majors behind                    | `Warning`             |
| Outdated, ≥ `escalate_at_majors` behind       | `Error` (failing)     |

By default (`escalate_at_majors` unset) outdated findings produce
`Warn`-verdict checks only. The `escalate_at_majors(n)` builder
escalates findings to a failing `CheckResult` once they are at least
`n` majors behind. The threshold MUST be configurable per-check.

## 6. JSON wire format

`DepResult`, `UnusedDep`, `OutdatedDep`, `DepScope`, and `DepKind`
MUST be serializable via `serde_json`. Field names MUST use
`snake_case`. Enum variants MUST use `lowercase`. Optional fields
with default values (e.g. `kind = None`, `escalate_at_majors = None`)
MUST be omitted on serialization.

## 7. Producer contract

`DepProducer::produce()` MUST always return a `Report`. It MUST NOT
panic on subprocess failure; instead, it MUST emit a single
`CheckResult::fail("deps::health", Severity::Critical)` carrying the
error message in `detail` and the tags `deps` + `subprocess`.

On success, the produced `Report` MUST contain one `CheckResult` per
finding, named `deps::unused::<crate>` or `deps::outdated::<crate>`,
tagged `deps` plus the kind-specific tag (`unused` / `outdated`).

## 8. Stability

Through `0.9.x` the public API MAY shift. The `1.0` release pins the
API and the severity policy table above. The wire format of
`DepResult`, `UnusedDep`, `OutdatedDep`, and the JSON shape emitted
through `dev-report` MUST stay stable from `1.0` onward.
