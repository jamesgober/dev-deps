//! # dev-deps
//!
//! Dependency health checking for Rust. Detects unused, outdated, and
//! policy-violating dependencies. Part of the `dev-*` verification suite.
//!
//! Wraps `cargo-udeps` (unused dependencies) and `cargo-outdated`
//! (out-of-date versions). Emits findings as `dev-report::Report`.
//!
//! ## What it checks
//!
//! - **Unused dependencies**: declared in `Cargo.toml` but never imported.
//! - **Outdated versions**: a newer version exists on crates.io.
//! - **Major-version lag**: how many major versions behind we are.
//!
//! ## Quick example
//!
//! ```no_run
//! use dev_deps::{DepCheck, DepScope};
//!
//! let check = DepCheck::new("my-crate", "0.1.0").scope(DepScope::All);
//! let result = check.execute().unwrap();
//! let report = result.into_report();
//! ```
//!
//! ## Status
//!
//! Pre-1.0. API shape defined; subprocess integration lands in `0.9.1`.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use dev_report::{CheckResult, Report, Severity};

/// Scope of a dependency check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepScope {
    /// Run only the unused-dependency scanner (cargo-udeps).
    Unused,
    /// Run only the outdated-version scanner (cargo-outdated).
    Outdated,
    /// Run both.
    All,
}

/// Configuration for a dependency check.
#[derive(Debug, Clone)]
pub struct DepCheck {
    name: String,
    version: String,
    scope: DepScope,
}

impl DepCheck {
    /// Begin a new dependency check.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            scope: DepScope::All,
        }
    }

    /// Set the check scope.
    pub fn scope(mut self, scope: DepScope) -> Self {
        self.scope = scope;
        self
    }

    /// Selected scope.
    pub fn dep_scope(&self) -> DepScope {
        self.scope
    }

    /// Execute the check.
    ///
    /// In `0.9.0` this is a stub; subprocess integration lands in `0.9.1`.
    pub fn execute(&self) -> Result<DepResult, DepError> {
        Ok(DepResult {
            name: self.name.clone(),
            version: self.version.clone(),
            scope: self.scope,
            unused: Vec::new(),
            outdated: Vec::new(),
        })
    }
}

/// An unused dependency finding.
#[derive(Debug, Clone)]
pub struct UnusedDep {
    /// Crate name as declared in `Cargo.toml`.
    pub crate_name: String,
    /// Dependency kind: "dependencies", "dev-dependencies", "build-dependencies".
    pub kind: String,
}

/// An outdated dependency finding.
#[derive(Debug, Clone)]
pub struct OutdatedDep {
    /// Crate name.
    pub crate_name: String,
    /// Current version pinned in `Cargo.toml`.
    pub current: String,
    /// Latest available version on the registry.
    pub latest: String,
    /// How many major versions behind we are.
    pub major_behind: u32,
}

/// Result of a dependency check.
#[derive(Debug, Clone)]
pub struct DepResult {
    /// Crate name.
    pub name: String,
    /// Crate version.
    pub version: String,
    /// Scope that produced this result.
    pub scope: DepScope,
    /// Unused dependencies found.
    pub unused: Vec<UnusedDep>,
    /// Outdated dependencies found.
    pub outdated: Vec<OutdatedDep>,
}

impl DepResult {
    /// Total findings across all categories.
    pub fn total_findings(&self) -> usize {
        self.unused.len() + self.outdated.len()
    }

    /// Convert this result into a `dev-report::Report`.
    pub fn into_report(self) -> Report {
        let mut report = Report::new(&self.name, &self.version).with_producer("dev-deps");
        if self.total_findings() == 0 {
            report.push(CheckResult::pass("deps::health"));
        } else {
            for u in &self.unused {
                report.push(
                    CheckResult::warn(format!("deps::unused::{}", u.crate_name), Severity::Warning)
                        .with_detail(format!("unused in {}", u.kind)),
                );
            }
            for o in &self.outdated {
                let sev = if o.major_behind >= 2 {
                    Severity::Warning
                } else {
                    Severity::Info
                };
                report.push(
                    CheckResult::warn(format!("deps::outdated::{}", o.crate_name), sev)
                        .with_detail(format!(
                            "{} -> {} ({} major behind)",
                            o.current, o.latest, o.major_behind
                        )),
                );
            }
        }
        report.finish();
        report
    }
}

/// Errors that can arise during a dependency check.
#[derive(Debug)]
pub enum DepError {
    /// `cargo-udeps` is not installed.
    UdepsToolNotInstalled,
    /// `cargo-outdated` is not installed.
    OutdatedToolNotInstalled,
    /// Subprocess failure.
    SubprocessFailed(String),
    /// Output parsing failure.
    ParseError(String),
}

impl std::fmt::Display for DepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UdepsToolNotInstalled => write!(f, "cargo-udeps is not installed"),
            Self::OutdatedToolNotInstalled => write!(f, "cargo-outdated is not installed"),
            Self::SubprocessFailed(s) => write!(f, "subprocess failed: {s}"),
            Self::ParseError(s) => write!(f, "parse error: {s}"),
        }
    }
}

impl std::error::Error for DepError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_builds() {
        let c = DepCheck::new("x", "0.1.0").scope(DepScope::Unused);
        assert_eq!(c.dep_scope(), DepScope::Unused);
    }

    #[test]
    fn empty_result_passes() {
        let r = DepResult {
            name: "x".into(),
            version: "0.1.0".into(),
            scope: DepScope::All,
            unused: Vec::new(),
            outdated: Vec::new(),
        };
        assert_eq!(r.total_findings(), 0);
        let report = r.into_report();
        assert!(report.passed());
    }

    #[test]
    fn findings_produce_warnings() {
        let r = DepResult {
            name: "x".into(),
            version: "0.1.0".into(),
            scope: DepScope::All,
            unused: vec![UnusedDep {
                crate_name: "foo".into(),
                kind: "dependencies".into(),
            }],
            outdated: vec![OutdatedDep {
                crate_name: "bar".into(),
                current: "1.0.0".into(),
                latest: "3.0.0".into(),
                major_behind: 2,
            }],
        };
        let report = r.into_report();
        // Both are Warn verdicts, so overall is Warn.
        assert!(report.warned());
    }
}
