# dev-deps — API Reference

> Hand-written reference. Mirrors `cargo doc --open` output with
> curated examples and structure.

## Table of contents

- [`DepScope`](#depscope)
- [`DepCheck`](#depcheck)
  - [`DepCheck::new`](#depchecknew)
  - [`DepCheck::scope`](#depcheckscope)
  - [`DepCheck::dep_scope`](#depcheckdep_scope)
  - [`DepCheck::execute`](#depcheckexecute)
- [`UnusedDep`](#unuseddep)
- [`OutdatedDep`](#outdateddep)
- [`DepResult`](#depresult)
  - [Fields](#depresult-fields)
  - [`DepResult::total_findings`](#depresulttotal_findings)
  - [`DepResult::into_report`](#depresultinto_report)
- [`DepError`](#deperror)

---

## `DepScope`

```rust
pub enum DepScope {
    Unused,
    Outdated,
    All,
}
```

Selects which checks to run.

```rust
use dev_deps::DepScope;
let s = DepScope::All;
assert_eq!(s, DepScope::All);
```

---

## `DepCheck`

```rust
pub struct DepCheck { /* private */ }
```

### `DepCheck::new`

```rust
pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self
```

| Parameter | Type                | Description       |
|-----------|---------------------|-------------------|
| `name`    | `impl Into<String>` | Crate name.       |
| `version` | `impl Into<String>` | Crate version.    |

```rust
use dev_deps::DepCheck;
let c = DepCheck::new("my-crate", "0.1.0");
```

### `DepCheck::scope`

```rust
pub fn scope(self, scope: DepScope) -> Self
```

Set the audit scope.

```rust
use dev_deps::{DepCheck, DepScope};
let c = DepCheck::new("my-crate", "0.1.0").scope(DepScope::Unused);
```

### `DepCheck::dep_scope`

```rust
pub fn dep_scope(&self) -> DepScope
```

Return the configured scope.

### `DepCheck::execute`

```rust
pub fn execute(&self) -> Result<DepResult, DepError>
```

Run the configured checks via `cargo-udeps` and/or `cargo-outdated`.

---

## `UnusedDep`

```rust
pub struct UnusedDep {
    pub crate_name: String,
    pub kind: String,    // "dependencies" | "dev-dependencies" | "build-dependencies"
}
```

A declared dependency that is never imported in the relevant kind's
source set.

---

## `OutdatedDep`

```rust
pub struct OutdatedDep {
    pub crate_name: String,
    pub current: String,
    pub latest: String,
    pub major_behind: u32,
}
```

A dependency where a newer version exists on the registry.

---

## `DepResult`

```rust
pub struct DepResult {
    pub name: String,
    pub version: String,
    pub scope: DepScope,
    pub unused: Vec<UnusedDep>,
    pub outdated: Vec<OutdatedDep>,
}
```

### DepResult fields

| Field      | Type                | Description                            |
|------------|---------------------|----------------------------------------|
| `name`     | `String`            | Crate name.                            |
| `version`  | `String`            | Crate version.                         |
| `scope`    | `DepScope`          | Scope that produced this result.       |
| `unused`   | `Vec<UnusedDep>`    | Unused dependencies found.             |
| `outdated` | `Vec<OutdatedDep>`  | Outdated dependencies found.           |

### `DepResult::total_findings`

```rust
pub fn total_findings(&self) -> usize
```

Total count across all categories.

### `DepResult::into_report`

```rust
pub fn into_report(self) -> Report
```

Convert findings to a `dev-report::Report`. Empty findings produces a
passing check; any findings produce per-crate Warn-verdict checks.
Severity follows the table in REPS § 5.

---

## `DepError`

```rust
pub enum DepError {
    UdepsToolNotInstalled,
    OutdatedToolNotInstalled,
    SubprocessFailed(String),
    ParseError(String),
}
```

Errors that can arise during check execution. `UdepsToolNotInstalled`
remediation is `cargo install cargo-udeps`. `OutdatedToolNotInstalled`
remediation is `cargo install cargo-outdated`.
