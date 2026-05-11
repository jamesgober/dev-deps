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
</p>

<p align="center">
    Unused, outdated, and policy-violating dependencies.<br>
    Part of the <code>dev-*</code> verification suite.
</p>

---

## What it does

`dev-deps` answers three questions about your dependency tree:

- Are any declared dependencies actually **unused**?
- Are any **outdated**, and by how many major versions?
- Are any flagged by policy (license, banned, source)?

Wraps `cargo-udeps` and `cargo-outdated`. Emits findings as
`dev-report::Report` so AI agents and CI gates can act on them
programmatically.

## Quick start

```toml
[dependencies]
dev-deps = "0.9"
```

```rust
use dev_deps::{DepCheck, DepScope};

let check = DepCheck::new("my-crate", "0.1.0").scope(DepScope::All);
let result = check.execute()?;
let report = result.into_report();
println!("{}", report.to_json()?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Requirements

```bash
cargo install cargo-udeps cargo-outdated
```

`cargo-udeps` requires nightly Rust. The crate handles that transparently
when it shells out.

## Scopes

| Scope       | What it runs                                       |
|-------------|-----------------------------------------------------|
| `Unused`    | `cargo-udeps` only.                                 |
| `Outdated`  | `cargo-outdated` only.                              |
| `All`       | Both.                                                |

## Severity assignment

| Finding                  | Severity              |
|--------------------------|-----------------------|
| Unused dependency        | `Warning`             |
| Outdated, 0-1 major behind | `Info`              |
| Outdated, 2+ major behind  | `Warning`           |

Verdicts are `Warn`, not `Fail`, by default — dependency health is
advisory, not blocking. You can configure this via thresholds in
later releases.

## The `dev-*` suite

See [`dev-tools`](https://github.com/jamesgober/dev-tools) for the
full suite.

## Status

`v0.9.0` is the foundation release: API shape defined, subprocess
integration lands in `0.9.1`. Production use is discouraged until
`1.0`.

## Minimum supported Rust version

`1.85` — pinned in `Cargo.toml` and verified by CI.

## License

Apache-2.0. See [LICENSE](LICENSE).
