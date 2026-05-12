//! # dev-deps
//!
//! Dependency health checking for Rust. Detects unused and outdated
//! dependencies. Part of the `dev-*` verification suite.
//!
//! Wraps [`cargo-udeps`][cargo-udeps] (unused dependencies) and
//! [`cargo-outdated`][cargo-outdated] (out-of-date versions). Emits
//! findings as [`dev_report::Report`].
//!
//! ## What it checks
//!
//! - **Unused dependencies** — declared in `Cargo.toml` but never imported.
//! - **Outdated versions** — a newer version exists on crates.io.
//! - **Major-version lag** — how many major versions behind the current pin is.
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
//! ## Requirements
//!
//! ```text
//! cargo install cargo-udeps cargo-outdated
//! rustup toolchain install nightly      # cargo-udeps requires nightly
//! ```
//!
//! [cargo-udeps]: https://crates.io/crates/cargo-udeps
//! [cargo-outdated]: https://crates.io/crates/cargo-outdated

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::path::PathBuf;

use dev_report::{CheckResult, Evidence, Report, Severity};
use serde::{Deserialize, Serialize};

mod outdated;
mod producer;
mod udeps;

pub use producer::DepProducer;

// ---------------------------------------------------------------------------
// DepScope
// ---------------------------------------------------------------------------

/// Scope of a dependency check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DepScope {
    /// Run only the unused-dependency scanner (`cargo udeps`).
    Unused,
    /// Run only the outdated-version scanner (`cargo outdated`).
    Outdated,
    /// Run both.
    All,
}

impl DepScope {
    fn runs_unused(self) -> bool {
        matches!(self, Self::Unused | Self::All)
    }

    fn runs_outdated(self) -> bool {
        matches!(self, Self::Outdated | Self::All)
    }
}

// ---------------------------------------------------------------------------
// DepKind
// ---------------------------------------------------------------------------

/// Where a dependency is declared in `Cargo.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DepKind {
    /// `[dependencies]`.
    Normal,
    /// `[dev-dependencies]`.
    Development,
    /// `[build-dependencies]`.
    Build,
}

impl DepKind {
    /// Human-readable label, matching the `Cargo.toml` section name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "dependencies",
            Self::Development => "dev-dependencies",
            Self::Build => "build-dependencies",
        }
    }
}

// ---------------------------------------------------------------------------
// DepCheck
// ---------------------------------------------------------------------------

/// Configuration for a dependency check.
///
/// # Example
///
/// ```no_run
/// use dev_deps::{DepCheck, DepScope};
/// use dev_report::Severity;
///
/// let check = DepCheck::new("my-crate", "0.1.0")
///     .scope(DepScope::All)
///     .workspace()
///     .allow("legacy-shim")
///     .severity_threshold(Severity::Warning)
///     .escalate_at_majors(3);
///
/// let _result = check.execute().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct DepCheck {
    name: String,
    version: String,
    scope: DepScope,
    workdir: Option<PathBuf>,
    workspace: bool,
    excludes: Vec<String>,
    allow_list: Vec<String>,
    severity_threshold: Option<Severity>,
    escalate_at_majors: Option<u32>,
}

impl DepCheck {
    /// Begin a new dependency check.
    ///
    /// `name` and `version` are descriptive — they identify the subject
    /// in the produced `Report`.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            scope: DepScope::All,
            workdir: None,
            workspace: false,
            excludes: Vec::new(),
            allow_list: Vec::new(),
            severity_threshold: None,
            escalate_at_majors: None,
        }
    }

    /// Pick which checks to run. Defaults to [`DepScope::All`].
    pub fn scope(mut self, scope: DepScope) -> Self {
        self.scope = scope;
        self
    }

    /// Selected scope.
    pub fn dep_scope(&self) -> DepScope {
        self.scope
    }

    /// Run the subprocesses from `dir` instead of the current directory.
    pub fn in_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.workdir = Some(dir.into());
        self
    }

    /// Pass `--workspace` so every workspace member is checked.
    pub fn workspace(mut self) -> Self {
        self.workspace = true;
        self
    }

    /// Exclude a crate from both scanners. May be called multiple times.
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.excludes.push(pattern.into());
        self
    }

    /// Suppress findings against a specific crate name. Matches both
    /// unused and outdated findings on `crate_name`.
    pub fn allow(mut self, crate_name: impl Into<String>) -> Self {
        self.allow_list.push(crate_name.into());
        self
    }

    /// Bulk version of [`allow`](Self::allow).
    pub fn allow_all<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allow_list.extend(names.into_iter().map(Into::into));
        self
    }

    /// Discard findings whose severity is *below* `threshold`. Findings
    /// at or above the threshold are kept.
    pub fn severity_threshold(mut self, threshold: Severity) -> Self {
        self.severity_threshold = Some(threshold);
        self
    }

    /// Escalate outdated findings to a *failing* check when the crate
    /// is at least `n` major versions behind. By default, outdated
    /// findings produce only `Warn`-verdict checks.
    pub fn escalate_at_majors(mut self, n: u32) -> Self {
        self.escalate_at_majors = Some(n);
        self
    }

    /// Subject name passed in via [`new`](Self::new).
    pub fn subject(&self) -> &str {
        &self.name
    }

    /// Subject version passed in via [`new`](Self::new).
    pub fn subject_version(&self) -> &str {
        &self.version
    }

    /// Execute the check.
    ///
    /// Each enabled tool is invoked as a subprocess. Findings are
    /// filtered through the allow-list and severity threshold, then
    /// sorted by `crate_name` for determinism.
    pub fn execute(&self) -> Result<DepResult, DepError> {
        let mut unused: Vec<UnusedDep> = Vec::new();
        let mut outdated: Vec<OutdatedDep> = Vec::new();

        if self.scope.runs_unused() {
            unused = udeps::run(self.workdir.as_deref(), self.workspace)?;
        }
        if self.scope.runs_outdated() {
            outdated = outdated::run(self.workdir.as_deref(), self.workspace, &self.excludes)?;
        }

        if !self.allow_list.is_empty() {
            unused.retain(|u| !self.allow_list.iter().any(|n| n == &u.crate_name));
            outdated.retain(|o| !self.allow_list.iter().any(|n| n == &o.crate_name));
        }
        if !self.excludes.is_empty() {
            unused.retain(|u| !self.excludes.iter().any(|n| n == &u.crate_name));
        }

        if let Some(threshold) = self.severity_threshold {
            let t = severity_ord(threshold);
            unused.retain(|u| severity_ord(u.severity()) >= t);
            outdated.retain(|o| severity_ord(o.severity(self.escalate_at_majors)) >= t);
        }

        unused.sort_by(|a, b| a.crate_name.cmp(&b.crate_name).then(a.kind.cmp(&b.kind)));
        outdated.sort_by(|a, b| a.crate_name.cmp(&b.crate_name));
        unused.dedup_by(|a, b| a.crate_name == b.crate_name && a.kind == b.kind);
        outdated.dedup_by(|a, b| a.crate_name == b.crate_name);

        Ok(DepResult {
            name: self.name.clone(),
            version: self.version.clone(),
            scope: self.scope,
            unused,
            outdated,
            escalate_at_majors: self.escalate_at_majors,
        })
    }
}

// ---------------------------------------------------------------------------
// UnusedDep + OutdatedDep
// ---------------------------------------------------------------------------

/// An unused-dependency finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedDep {
    /// Crate name as declared in `Cargo.toml`.
    pub crate_name: String,
    /// Where the dependency is declared.
    pub kind: DepKind,
}

impl UnusedDep {
    /// Severity assigned to this finding (always `Warning`).
    pub fn severity(&self) -> Severity {
        Severity::Warning
    }
}

/// An outdated-dependency finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutdatedDep {
    /// Crate name.
    pub crate_name: String,
    /// Current version pinned in `Cargo.toml`.
    pub current: String,
    /// Latest available version on the registry.
    pub latest: String,
    /// How many major versions behind the current pin is.
    pub major_behind: u32,
    /// Where the dependency is declared, when the underlying tool exposed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<DepKind>,
}

impl OutdatedDep {
    /// Severity assigned to this finding.
    ///
    /// `Info` for 0–1 major behind; `Warning` for 2+ majors behind.
    /// When `escalate_at` is `Some(n)` and the crate is at least `n`
    /// majors behind, the severity is bumped to `Error` so the produced
    /// `CheckResult` becomes a failing verdict.
    pub fn severity(&self, escalate_at: Option<u32>) -> Severity {
        if let Some(n) = escalate_at {
            if self.major_behind >= n {
                return Severity::Error;
            }
        }
        if self.major_behind >= 2 {
            Severity::Warning
        } else {
            Severity::Info
        }
    }
}

// ---------------------------------------------------------------------------
// DepResult
// ---------------------------------------------------------------------------

/// Result of a dependency check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepResult {
    /// Subject name.
    pub name: String,
    /// Subject version.
    pub version: String,
    /// Scope that produced this result.
    pub scope: DepScope,
    /// Unused dependencies.
    pub unused: Vec<UnusedDep>,
    /// Outdated dependencies.
    pub outdated: Vec<OutdatedDep>,
    /// Major-version escalation threshold that was active when the
    /// result was produced. Carried so `into_report` can re-derive the
    /// severity each finding would have had at execution time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalate_at_majors: Option<u32>,
}

impl DepResult {
    /// Total findings across both categories.
    pub fn total_findings(&self) -> usize {
        self.unused.len() + self.outdated.len()
    }

    /// Number of unused-dependency findings.
    pub fn unused_count(&self) -> usize {
        self.unused.len()
    }

    /// Number of outdated-dependency findings.
    pub fn outdated_count(&self) -> usize {
        self.outdated.len()
    }

    /// Highest severity present across all findings, if any.
    pub fn worst_severity(&self) -> Option<Severity> {
        let mut worst: Option<Severity> = None;
        let mut bump = |s: Severity| {
            worst = Some(match worst {
                None => s,
                Some(prev) if severity_ord(s) > severity_ord(prev) => s,
                Some(prev) => prev,
            });
        };
        for u in &self.unused {
            bump(u.severity());
        }
        for o in &self.outdated {
            bump(o.severity(self.escalate_at_majors));
        }
        worst
    }

    /// Convert this result into a [`dev_report::Report`].
    ///
    /// No findings → one `CheckResult::pass("deps::health")`. Otherwise
    /// one `CheckResult` per finding, named `deps::unused::<crate>` or
    /// `deps::outdated::<crate>`, tagged `deps` plus a kind-specific
    /// tag (`unused` or `outdated`). Outdated findings carry numeric
    /// evidence for `major_behind`.
    pub fn into_report(self) -> Report {
        let mut report = Report::new(&self.name, &self.version).with_producer("dev-deps");
        if self.total_findings() == 0 {
            report.push(
                CheckResult::pass("deps::health")
                    .with_tag("deps")
                    .with_detail(format!("{} scope: no findings", scope_label(self.scope))),
            );
        } else {
            for u in &self.unused {
                let check =
                    CheckResult::warn(format!("deps::unused::{}", u.crate_name), u.severity())
                        .with_detail(format!("unused in {}", u.kind.as_str()))
                        .with_tag("deps")
                        .with_tag("unused")
                        .with_evidence(Evidence::kv(
                            "finding",
                            [("crate", u.crate_name.as_str()), ("kind", u.kind.as_str())],
                        ));
                report.push(check);
            }
            for o in &self.outdated {
                let sev = o.severity(self.escalate_at_majors);
                let kind = if sev == Severity::Error {
                    // Escalated; produce a failing verdict.
                    CheckResult::fail(format!("deps::outdated::{}", o.crate_name), sev)
                } else {
                    CheckResult::warn(format!("deps::outdated::{}", o.crate_name), sev)
                };
                let mut check = kind
                    .with_detail(format!(
                        "{} -> {} ({} major behind)",
                        o.current, o.latest, o.major_behind
                    ))
                    .with_tag("deps")
                    .with_tag("outdated")
                    .with_evidence(Evidence::numeric_int("major_behind", o.major_behind as i64));
                let mut kv: Vec<(String, String)> = vec![
                    ("crate".into(), o.crate_name.clone()),
                    ("current".into(), o.current.clone()),
                    ("latest".into(), o.latest.clone()),
                ];
                if let Some(k) = o.kind {
                    kv.push(("kind".into(), k.as_str().into()));
                }
                check = check.with_evidence(Evidence::kv("finding", kv));
                report.push(check);
            }
        }
        report.finish();
        report
    }
}

fn scope_label(s: DepScope) -> &'static str {
    match s {
        DepScope::Unused => "unused",
        DepScope::Outdated => "outdated",
        DepScope::All => "all",
    }
}

pub(crate) fn severity_ord(s: Severity) -> u8 {
    match s {
        Severity::Info => 0,
        Severity::Warning => 1,
        Severity::Error => 2,
        Severity::Critical => 3,
    }
}

// ---------------------------------------------------------------------------
// DepError
// ---------------------------------------------------------------------------

/// Errors that can arise during a dependency check.
#[derive(Debug)]
pub enum DepError {
    /// `cargo-udeps` (or the nightly toolchain it needs) is not installed.
    UdepsToolNotInstalled,
    /// `cargo-outdated` is not installed.
    OutdatedToolNotInstalled,
    /// Subprocess failure with the captured stderr.
    SubprocessFailed(String),
    /// Output parsing failure.
    ParseError(String),
}

impl std::fmt::Display for DepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UdepsToolNotInstalled => write!(
                f,
                "cargo-udeps is not installed (or nightly toolchain missing); run `cargo install cargo-udeps` and `rustup toolchain install nightly`"
            ),
            Self::OutdatedToolNotInstalled => write!(
                f,
                "cargo-outdated is not installed; run `cargo install cargo-outdated`"
            ),
            Self::SubprocessFailed(s) => write!(f, "dependency check subprocess failed: {s}"),
            Self::ParseError(s) => write!(f, "could not parse subprocess output: {s}"),
        }
    }
}

impl std::error::Error for DepError {}

#[cfg(test)]
mod tests {
    use super::*;
    use dev_report::Verdict;

    fn unused(name: &str, kind: DepKind) -> UnusedDep {
        UnusedDep {
            crate_name: name.into(),
            kind,
        }
    }

    fn outdated(name: &str, cur: &str, latest: &str, major_behind: u32) -> OutdatedDep {
        OutdatedDep {
            crate_name: name.into(),
            current: cur.into(),
            latest: latest.into(),
            major_behind,
            kind: Some(DepKind::Normal),
        }
    }

    fn make_result(unused_: Vec<UnusedDep>, outdated_: Vec<OutdatedDep>) -> DepResult {
        DepResult {
            name: "x".into(),
            version: "0.1.0".into(),
            scope: DepScope::All,
            unused: unused_,
            outdated: outdated_,
            escalate_at_majors: None,
        }
    }

    #[test]
    fn empty_findings_produces_passing_report() {
        let r = make_result(Vec::new(), Vec::new());
        let report = r.into_report();
        assert!(report.passed());
    }

    #[test]
    fn unused_findings_produce_warn_verdict() {
        let r = make_result(vec![unused("legacy", DepKind::Normal)], Vec::new());
        let report = r.into_report();
        assert!(report.warned());
        assert_eq!(report.checks.len(), 1);
        let c = &report.checks[0];
        assert!(c.has_tag("deps") && c.has_tag("unused"));
        assert_eq!(c.name, "deps::unused::legacy");
    }

    #[test]
    fn outdated_one_major_is_info() {
        let r = make_result(Vec::new(), vec![outdated("foo", "1.0.0", "2.0.0", 1)]);
        let sev = r.outdated[0].severity(None);
        assert_eq!(sev, Severity::Info);
    }

    #[test]
    fn outdated_two_or_more_majors_is_warning() {
        let r = make_result(Vec::new(), vec![outdated("foo", "1.0.0", "3.0.0", 2)]);
        assert_eq!(r.outdated[0].severity(None), Severity::Warning);
    }

    #[test]
    fn escalate_at_majors_bumps_to_error_and_fail() {
        let mut r = make_result(Vec::new(), vec![outdated("foo", "1.0.0", "5.0.0", 4)]);
        r.escalate_at_majors = Some(3);
        assert_eq!(r.outdated[0].severity(Some(3)), Severity::Error);
        let report = r.into_report();
        // Error severity on outdated means CheckResult::fail; overall failed.
        assert!(report.failed());
        let c = &report.checks[0];
        assert_eq!(c.verdict, Verdict::Fail);
    }

    #[test]
    fn escalate_does_not_fire_below_threshold() {
        let mut r = make_result(Vec::new(), vec![outdated("foo", "1.0.0", "2.0.0", 1)]);
        r.escalate_at_majors = Some(3);
        assert_eq!(r.outdated[0].severity(Some(3)), Severity::Info);
    }

    #[test]
    fn total_findings_sums_both_categories() {
        let r = make_result(
            vec![
                unused("a", DepKind::Normal),
                unused("b", DepKind::Development),
            ],
            vec![outdated("c", "1.0.0", "2.0.0", 1)],
        );
        assert_eq!(r.total_findings(), 3);
        assert_eq!(r.unused_count(), 2);
        assert_eq!(r.outdated_count(), 1);
    }

    #[test]
    fn worst_severity_picks_max_across_categories() {
        let mut r = make_result(
            vec![unused("a", DepKind::Normal)],
            vec![outdated("c", "1.0.0", "3.0.0", 2)],
        );
        r.escalate_at_majors = Some(2);
        assert_eq!(r.worst_severity(), Some(Severity::Error));
    }

    #[test]
    fn result_round_trips_through_json() {
        let r = make_result(
            vec![unused("a", DepKind::Normal)],
            vec![outdated("c", "1.0.0", "2.0.0", 1)],
        );
        let s = serde_json::to_string(&r).unwrap();
        let back: DepResult = serde_json::from_str(&s).unwrap();
        assert_eq!(back.unused.len(), 1);
        assert_eq!(back.outdated.len(), 1);
    }

    #[test]
    fn check_builder_chains() {
        let c = DepCheck::new("x", "0.1.0")
            .scope(DepScope::Outdated)
            .workspace()
            .exclude("ignored")
            .allow("legacy-shim")
            .allow_all(["a", "b"])
            .severity_threshold(Severity::Warning)
            .escalate_at_majors(3);
        assert_eq!(c.dep_scope(), DepScope::Outdated);
        assert_eq!(c.subject(), "x");
        assert_eq!(c.subject_version(), "0.1.0");
    }

    #[test]
    fn depkind_label_matches_cargo_toml() {
        assert_eq!(DepKind::Normal.as_str(), "dependencies");
        assert_eq!(DepKind::Development.as_str(), "dev-dependencies");
        assert_eq!(DepKind::Build.as_str(), "build-dependencies");
    }

    #[test]
    fn dep_scope_runs_helpers() {
        assert!(DepScope::All.runs_unused());
        assert!(DepScope::All.runs_outdated());
        assert!(DepScope::Unused.runs_unused());
        assert!(!DepScope::Unused.runs_outdated());
        assert!(DepScope::Outdated.runs_outdated());
        assert!(!DepScope::Outdated.runs_unused());
    }
}
