//! Public-API smoke tests.
//!
//! The real-subprocess path is exercised only when both `cargo-udeps`
//! and `cargo-outdated` are installed and `CARGO_TARGET_DIR` points
//! outside the workspace; see the `#[ignore]`d test at the bottom.

use dev_deps::{DepCheck, DepKind, DepResult, DepScope, OutdatedDep, UnusedDep};
use dev_report::Severity;

fn make_result(unused: Vec<UnusedDep>, outdated: Vec<OutdatedDep>) -> DepResult {
    DepResult {
        name: "x".into(),
        version: "0.1.0".into(),
        scope: DepScope::All,
        unused,
        outdated,
        escalate_at_majors: None,
    }
}

#[test]
fn default_scope_is_all() {
    let c = DepCheck::new("x", "0.1.0");
    assert_eq!(c.dep_scope(), DepScope::All);
}

#[test]
fn scope_selection_round_trips() {
    let c = DepCheck::new("x", "0.1.0").scope(DepScope::Unused);
    assert_eq!(c.dep_scope(), DepScope::Unused);
}

#[test]
fn check_accessors_round_trip_subject() {
    let c = DepCheck::new("alpha", "1.2.3");
    assert_eq!(c.subject(), "alpha");
    assert_eq!(c.subject_version(), "1.2.3");
}

#[test]
fn check_builder_chains() {
    let _c = DepCheck::new("x", "0.1.0")
        .scope(DepScope::Outdated)
        .workspace()
        .exclude("legacy")
        .allow("legacy-shim")
        .allow_all(["a", "b"])
        .severity_threshold(Severity::Warning)
        .escalate_at_majors(3);
}

#[test]
fn empty_result_total_findings() {
    let r = make_result(Vec::new(), Vec::new());
    assert_eq!(r.total_findings(), 0);
    assert_eq!(r.unused_count(), 0);
    assert_eq!(r.outdated_count(), 0);
}

#[test]
fn empty_result_passes() {
    let r = make_result(Vec::new(), Vec::new());
    let report = r.into_report();
    assert!(report.passed());
}

#[test]
fn unused_produces_warn_report() {
    let r = make_result(
        vec![UnusedDep {
            crate_name: "foo".into(),
            kind: DepKind::Normal,
        }],
        Vec::new(),
    );
    let report = r.into_report();
    assert!(report.warned());
}

#[test]
fn mixed_findings_count_is_two() {
    let r = make_result(
        vec![UnusedDep {
            crate_name: "foo".into(),
            kind: DepKind::Normal,
        }],
        vec![OutdatedDep {
            crate_name: "bar".into(),
            current: "1.0.0".into(),
            latest: "2.0.0".into(),
            major_behind: 1,
            kind: Some(DepKind::Normal),
        }],
    );
    assert_eq!(r.total_findings(), 2);
}

#[test]
fn outdated_severity_picks_info_for_one_major() {
    let dep = OutdatedDep {
        crate_name: "x".into(),
        current: "1.0.0".into(),
        latest: "2.0.0".into(),
        major_behind: 1,
        kind: None,
    };
    assert_eq!(dep.severity(None), Severity::Info);
}

#[test]
fn outdated_severity_picks_warning_for_two_plus_majors() {
    let dep = OutdatedDep {
        crate_name: "x".into(),
        current: "1.0.0".into(),
        latest: "3.0.0".into(),
        major_behind: 2,
        kind: None,
    };
    assert_eq!(dep.severity(None), Severity::Warning);
}

#[test]
fn escalate_threshold_turns_outdated_into_error() {
    let dep = OutdatedDep {
        crate_name: "x".into(),
        current: "1.0.0".into(),
        latest: "5.0.0".into(),
        major_behind: 4,
        kind: None,
    };
    assert_eq!(dep.severity(Some(3)), Severity::Error);
}

/// Real subprocess test. Skipped by default.
///
/// Requires `cargo-udeps` (nightly toolchain too) + `cargo-outdated`
/// installed, and `CARGO_TARGET_DIR` pointing outside the workspace
/// so the inner cargo invocations don't fight the outer `cargo test`
/// for the workspace target-dir lock.
///
/// ```text
/// CARGO_TARGET_DIR=/tmp/deps-target cargo test -- --ignored
/// ```
#[test]
#[ignore = "requires cargo-udeps + cargo-outdated + CARGO_TARGET_DIR outside the workspace"]
fn execute_against_real_tools() {
    let check = DepCheck::new("dev-deps", "0.9.0").scope(DepScope::All);
    let res = check
        .execute()
        .expect("cargo-udeps + cargo-outdated installed");
    let _ = res.into_report();
}
