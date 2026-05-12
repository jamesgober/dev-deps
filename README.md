<h1 align="center">
    <strong>dev-deps</strong>
    <br>
    <sup><sub>DEPENDENCY HEALTH FOR RUST</sub></sup>
</h1>

<p align="center">
    <a href="https://crates.io/crates/dev-deps"><img alt="crates.io" src="https://img.shields.io/crates/v/dev-deps.svg"></a>
    <a href="https://crates.io/crates/dev-deps"><img alt="downloads" src="https://img.shields.io/crates/d/dev-deps.svg"></a>
    <a href="https://docs.rs/dev-deps"><img alt="docs.rs" src="https://docs.rs/dev-deps/badge.svg"></a>
    <a href="https://github.com/jamesgober/dev-deps/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/jamesgober/dev-deps/actions/workflows/ci.yml/badge.svg"></a>
    <img alt="MSRV" src="https://img.shields.io/badge/msrv-1.85%2B-blue.svg?style=flat-square" title="Rust Version">
</p>

<p align="center">
    Unused and outdated dependencies.<br>
    Part of the <code>dev-*</code> verification suite.
</p>

---

## What it does

`dev-deps` answers two questions about your dependency tree:

- Are any declared dependencies actually **unused**?
- Are any **outdated**, and by how many major versions?

It wraps [`cargo-udeps`](https://crates.io/crates/cargo-udeps) and
[`cargo-outdated`](https://crates.io/crates/cargo-outdated) and emits
findings as a [`dev-report::Report`](https://docs.rs/dev-report) so AI
agents and CI gates can act on them programmatically.

## Quick start

```toml
[dependencies]
dev-deps = "0.9"
```

One-time tool install:

```bash
cargo install cargo-udeps cargo-outdated
rustup toolchain install nightly      # cargo-udeps requires nightly
```

Drive it from code:

```rust,no_run
use dev_deps::{DepCheck, DepScope};

let check = DepCheck::new("my-crate", "0.1.0").scope(DepScope::All);
let result = check.execute()?;
let report = result.into_report();
println!("{}", report.to_json()?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Scopes

| Scope                | What it runs                                      |
|----------------------|----------------------------------------------------|
| `DepScope::Unused`   | `cargo +nightly udeps --output json` only.        |
| `DepScope::Outdated` | `cargo outdated --format json` only.              |
| `DepScope::All`      | Both.                                              |

## Severity policy

| Finding                                       | `dev-report::Severity` |
|-----------------------------------------------|------------------------|
| Unused dependency                             | `Warning`              |
| Outdated, 0–1 major behind                    | `Info`                 |
| Outdated, 2+ majors behind                    | `Warning`              |
| Outdated, ≥ `escalate_at_majors` behind       | `Error` (failing)      |

By default, every finding is a `Warn`-verdict check — dependency
health is advisory, not blocking. Call `.escalate_at_majors(n)` on the
builder to make findings at least `n` majors behind produce a *failing*
`CheckResult` instead.

## Allow-list, exclude, and severity threshold

```rust,no_run
use dev_deps::{DepCheck, DepScope};
use dev_report::Severity;

let check = DepCheck::new("my-crate", "0.1.0")
    .scope(DepScope::All)
    .workspace()                          // pass --workspace to both tools
    .exclude("vendored-crate")            // skip a whole crate
    .allow("legacy-shim")                 // skip a single advisory ID / crate name
    .allow_all(["a", "b"])
    .severity_threshold(Severity::Warning) // drop Info findings
    .escalate_at_majors(3);                // fail when 3+ majors behind

let _result = check.execute()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## `Producer` integration

`DepProducer` plugs the check into a multi-producer pipeline driven
by [`dev-tools`](https://github.com/jamesgober/dev-tools):

```rust,no_run
use dev_deps::{DepCheck, DepProducer, DepScope};
use dev_report::Producer;

let producer = DepProducer::new(
    DepCheck::new("my-crate", "0.1.0").scope(DepScope::All),
);

let report = producer.produce();
println!("{}", report.to_json().unwrap());
```

Subprocess failures map to a single failing `CheckResult` named
`deps::health` with `Severity::Critical` — the pipeline keeps running.

## Wire format

`DepResult`, `UnusedDep`, `OutdatedDep`, `DepScope`, and `DepKind`
are all `serde`-derived. JSON output uses `snake_case` field names
and omits optional fields when they are `None`:

```json
{
  "name": "my-crate",
  "version": "0.1.0",
  "scope": "all",
  "unused": [
    { "crate_name": "legacy", "kind": "development" }
  ],
  "outdated": [
    {
      "crate_name": "serde",
      "current": "1.0.0",
      "latest": "2.0.0",
      "major_behind": 1,
      "kind": "normal"
    }
  ]
}
```

## Examples

| File                              | What it shows                                                   |
|-----------------------------------|------------------------------------------------------------------|
| `examples/basic.rs`               | Full check (`All` scope); graceful tool-missing handling.       |
| `examples/unused_only.rs`         | `Unused` scope only.                                            |
| `examples/outdated_only.rs`       | `Outdated` scope only.                                          |
| `examples/producer.rs`            | `DepProducer` (gated by `DEV_DEPS_EXAMPLE_RUN`).                |

## Requirements

Both tools must be installed:

```bash
cargo install cargo-udeps cargo-outdated
rustup toolchain install nightly      # cargo-udeps requires nightly
```

The crate detects absence of either tool and surfaces a typed
`DepError` variant rather than panicking.

Runtime dependency footprint: `dev-report`, `serde`, `serde_json`.

## Migration from `0.1.0`

`UnusedDep::kind` was a `String` in `0.1.0`; it is now a typed `DepKind`
enum. `OutdatedDep` also gained an optional `kind` field. If you
constructed these struct literals in `0.1.0`, update:

```rust
# use dev_deps::{DepKind, OutdatedDep, UnusedDep};
let _unused = UnusedDep {
    crate_name: "foo".into(),
    kind: DepKind::Normal,           // was: String
};

let _outdated = OutdatedDep {
    crate_name: "bar".into(),
    current: "1.0.0".into(),
    latest: "2.0.0".into(),
    major_behind: 1,
    kind: Some(DepKind::Normal),     // new in 0.9.0
};
```

The constructor surface (`DepCheck::new`, `DepScope` variants,
`DepResult::into_report`) is unchanged.

## The `dev-*` suite

See [`dev-tools`](https://github.com/jamesgober/dev-tools) for the
umbrella crate covering the full suite.

## Status

`v0.9.x` is the pre-1.0 stabilization line. The API is feature-complete
for unused-dependency detection, outdated-version detection, major-lag
escalation, allow-listing, and severity gating. Production use is fine;
`1.0` will pin the public API and the wire format.

## Minimum supported Rust version

`1.85` — pinned in `Cargo.toml` via `rust-version` and verified by
the MSRV job in CI.

## License

Apache-2.0. See [LICENSE](LICENSE).
