# dev-deps — Project Specification (REPS)

> Rust Engineering Project Specification.
> Normative language follows RFC 2119.

## 1. Purpose

`dev-deps` MUST check dependency health (unused, outdated, policy-
violating) and emit findings as `dev-report::Report`. Output MUST be
machine-readable so AI agents and CI gates can act on results.

## 2. Scope

This crate MUST provide:

- A `DepScope` enum (`Unused`, `Outdated`, `All`).
- A `DepCheck` builder.
- `UnusedDep` and `OutdatedDep` finding types.
- A `DepResult` with `into_report` integration.

This crate SHOULD provide (later versions):

- `cargo-udeps` subprocess integration (`0.9.1`).
- `cargo-outdated` subprocess integration (`0.9.1`).
- Major-version-lag threshold gating (`0.9.2`).
- Workspace-aware checks for multi-crate projects (`0.9.3`).

This crate MUST NOT:

- Replace `cargo-udeps` or `cargo-outdated`. We wrap them.
- Edit `Cargo.toml`. Reporting is the contract; remediation is the
  user's choice.
- Network-fetch package metadata directly. Tools handle that.

## 3. Determinism

Same project + same lockfile MUST produce the same findings list.
Order MUST be deterministic (sort by crate name).

## 4. Tool dependencies

`cargo-udeps` and `cargo-outdated` MUST be installed externally.
Detection of missing tools produces `DepError::UdepsToolNotInstalled`
or `DepError::OutdatedToolNotInstalled` with remediation guidance.

## 5. Severity policy

| Finding type                  | Severity              |
|-------------------------------|-----------------------|
| Unused dependency             | `Warning`             |
| Outdated, 0-1 major behind    | `Info`                |
| Outdated, 2+ majors behind    | `Warning`             |

All findings emit `Warn`-verdict checks by default. Configurable
escalation to `Fail` lands in `0.9.2`.

## 6. Stability

Through `0.9.x` the public API MAY shift. The `1.0` release pins the
API and the severity policy table above.
